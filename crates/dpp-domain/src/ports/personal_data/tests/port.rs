//! The personal-data port's contract, held to by its in-memory implementation.

use async_trait::async_trait;
use serde_json::json;

use super::super::stub::InMemoryPersonalData;
use super::super::{ErasureReceipt, HeldRecord, HeldSummary, PersonalDataPort};
use crate::error::DppError;
use crate::passport::PassportId;
use crate::personal_data::PersonalDataRecordId;

const FIELD: &str = "usageHistory.negativeEvents";

#[tokio::test]
async fn a_stored_record_is_held_under_the_identifier_it_was_given() {
    let port = InMemoryPersonalData::new();
    let passport = PassportId::new();
    let content = json!({ "driver": "named in the accident report" });
    let id = port.store(passport, FIELD, &content).await.unwrap();

    let Some(HeldRecord::Present(record)) = port.fetch(&id).await.unwrap() else {
        panic!("a stored record is present");
    };
    assert_eq!(record.id, id);
    assert_eq!(record.passport_id, passport);
    assert_eq!(record.field, FIELD);
    assert_eq!(record.content, content);
}

#[tokio::test]
async fn every_store_mints_a_new_identifier() {
    let port = InMemoryPersonalData::new();
    let passport = PassportId::new();
    let content = json!("same content");
    let first = port.store(passport, FIELD, &content).await.unwrap();
    let second = port.store(passport, FIELD, &content).await.unwrap();
    assert_ne!(first, second);
}

/// The identifier goes into a signed passport that outlives the record, so it
/// must not be made from what the record holds.
#[tokio::test]
async fn an_identifier_carries_nothing_of_the_content() {
    let port = InMemoryPersonalData::new();
    let id = port
        .store(PassportId::new(), FIELD, &json!("Jane Example"))
        .await
        .unwrap();
    assert!(!id.as_str().contains("Jane"));
}

#[tokio::test]
async fn erasure_keeps_a_tombstone_without_the_data() {
    let port = InMemoryPersonalData::new();
    let passport = PassportId::new();
    let id = port
        .store(passport, FIELD, &json!("Jane Example"))
        .await
        .unwrap();

    let receipt = port.erase(&id).await.unwrap();
    assert_eq!(receipt.record, id);
    assert_eq!(receipt.passport_id, passport);
    assert_eq!(receipt.field, FIELD);

    let held = port.fetch(&id).await.unwrap();
    assert_eq!(held, Some(HeldRecord::Erased(receipt)));
    assert!(
        !format!("{held:?}").contains("Jane"),
        "nothing of the content survives erasure"
    );
}

#[tokio::test]
async fn erasing_twice_returns_the_first_receipt() {
    let port = InMemoryPersonalData::new();
    let id = port
        .store(PassportId::new(), FIELD, &json!("x"))
        .await
        .unwrap();
    let first = port.erase(&id).await.unwrap();
    let again = port.erase(&id).await.unwrap();
    assert_eq!(first, again);
}

#[tokio::test]
async fn erasing_one_record_leaves_the_others() {
    let port = InMemoryPersonalData::new();
    let passport = PassportId::new();
    let erased = port.store(passport, FIELD, &json!("a")).await.unwrap();
    let kept = port.store(passport, FIELD, &json!("b")).await.unwrap();
    port.erase(&erased).await.unwrap();
    assert!(matches!(
        port.fetch(&kept).await.unwrap(),
        Some(HeldRecord::Present(_))
    ));
}

#[tokio::test]
async fn an_identifier_never_minted_is_unknown() {
    let port = InMemoryPersonalData::new();
    let stranger = PersonalDataRecordId::new("never-minted");
    assert_eq!(port.fetch(&stranger).await.unwrap(), None);
    assert!(matches!(
        port.erase(&stranger).await,
        Err(DppError::NotFound(_))
    ));
}

/// A `store` whose answer never arrived still left a record, and the caller
/// must be able to find it to erase it.
#[tokio::test]
async fn a_record_whose_identifier_was_lost_is_found_by_its_passport() {
    let port = InMemoryPersonalData::new();
    let passport = PassportId::new();
    let elsewhere = PassportId::new();
    let lost = port.store(passport, FIELD, &json!("a")).await.unwrap();
    let erased = port.store(passport, FIELD, &json!("b")).await.unwrap();
    port.store(elsewhere, FIELD, &json!("c")).await.unwrap();
    let receipt = port.erase(&erased).await.unwrap();

    let held = port.records_for(passport).await.unwrap();
    assert_eq!(
        held.len(),
        2,
        "only this passport's records, erased ones too"
    );
    let Some(HeldRecord::Present(first)) = held.first() else {
        panic!("the first record stored is listed first, with its data");
    };
    assert_eq!(first.id, lost);
    assert_eq!(held[1], HeldRecord::Erased(receipt));

    port.erase(&lost).await.unwrap();
    assert!(
        port.records_for(passport)
            .await
            .unwrap()
            .iter()
            .all(|h| matches!(h, HeldRecord::Erased(_)))
    );
}

#[tokio::test]
async fn a_passport_with_nothing_held_lists_nothing() {
    let port = InMemoryPersonalData::new();
    assert!(
        port.records_for(PassportId::new())
            .await
            .unwrap()
            .is_empty()
    );
}

/// A panic while the lock was held must not leave a record impossible to
/// erase.
#[tokio::test]
async fn a_poisoned_lock_still_lets_a_record_be_erased() {
    let port = std::sync::Arc::new(InMemoryPersonalData::new());
    let id = port
        .store(PassportId::new(), FIELD, &json!("x"))
        .await
        .unwrap();

    let poisoner = std::sync::Arc::clone(&port);
    let _ = std::thread::spawn(move || {
        let _guard = poisoner.held.lock().unwrap();
        panic!("poison the lock");
    })
    .join();
    assert!(port.held.is_poisoned());

    let receipt = port
        .erase(&id)
        .await
        .expect("erasure survives a poisoned lock");
    assert_eq!(receipt.record, id);
}

/// A receipt is put on a wire by whoever answers an erasure, so its keys are
/// fixed here: camelCase, the record as a bare string, nothing of the content.
#[test]
fn a_receipt_serialises_camel_case_and_reads_back() {
    use super::super::ErasureReceipt;
    use chrono::{TimeZone, Utc};

    let receipt = ErasureReceipt {
        record: PersonalDataRecordId::new("rec-1"),
        passport_id: PassportId::new(),
        field: FIELD.to_owned(),
        erased_at: Utc.with_ymd_and_hms(2026, 11, 23, 10, 0, 0).unwrap(),
    };

    let wire = serde_json::to_value(&receipt).expect("serialises");
    let mut keys: Vec<&str> = wire
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(keys, ["erasedAt", "field", "passportId", "record"]);
    assert_eq!(wire["record"], "rec-1");
    assert_eq!(wire["erasedAt"], "2026-11-23T10:00:00Z");

    let back: ErasureReceipt = serde_json::from_value(wire).expect("reads back");
    assert_eq!(back, receipt);
}

/// Answers the listing itself, and refuses every other read. A call that fell
/// through to the default `summaries_for` would reach `records_for` and panic.
struct AnswersItself(HeldSummary);

#[async_trait]
impl PersonalDataPort for AnswersItself {
    async fn store(
        &self,
        _: PassportId,
        _: &str,
        _: &serde_json::Value,
    ) -> Result<PersonalDataRecordId, DppError> {
        unreachable!()
    }

    async fn fetch(&self, _: &PersonalDataRecordId) -> Result<Option<HeldRecord>, DppError> {
        unreachable!()
    }

    async fn records_for(&self, _: PassportId) -> Result<Vec<HeldRecord>, DppError> {
        unreachable!()
    }

    async fn summaries_for(&self, _: PassportId) -> Result<Vec<HeldSummary>, DppError> {
        Ok(vec![self.0.clone()])
    }

    async fn erase(&self, _: &PersonalDataRecordId) -> Result<ErasureReceipt, DppError> {
        unreachable!()
    }
}

/// A listing carries the identifier, passport, field and time a record was
/// stored, and none of its content.
#[tokio::test]
async fn a_present_record_is_summarised_without_its_content() {
    let port = InMemoryPersonalData::new();
    let passport = PassportId::new();
    port.store(passport, FIELD, &json!("sentinel-content-9c1e"))
        .await
        .unwrap();

    let held = port.records_for(passport).await.unwrap();
    let HeldRecord::Present(record) = &held[0] else {
        panic!("a stored record is present");
    };
    let summaries = port.summaries_for(passport).await.unwrap();
    assert_eq!(
        summaries,
        [HeldSummary::Present {
            record: record.id.clone(),
            passport_id: record.passport_id,
            field: record.field.clone(),
            stored_at: record.stored_at,
        }]
    );
    assert!(
        !format!("{:?}", summaries[0]).contains("sentinel-content-9c1e"),
        "the summary carries none of the content"
    );
}

#[tokio::test]
async fn an_erased_record_is_summarised_as_its_receipt() {
    let port = InMemoryPersonalData::new();
    let passport = PassportId::new();
    let id = port.store(passport, FIELD, &json!("x")).await.unwrap();
    let receipt = port.erase(&id).await.unwrap();

    assert_eq!(
        port.summaries_for(passport).await.unwrap(),
        [HeldSummary::Erased(receipt)]
    );
}

#[tokio::test]
async fn summaries_come_in_the_order_stored_for_the_passport_asked_about() {
    let port = InMemoryPersonalData::new();
    let passport = PassportId::new();
    let elsewhere = PassportId::new();
    let first = port.store(passport, FIELD, &json!("a")).await.unwrap();
    let other = port.store(elsewhere, FIELD, &json!("b")).await.unwrap();
    let last = port.store(passport, FIELD, &json!("c")).await.unwrap();
    let receipt = port.erase(&first).await.unwrap();

    let summaries = port.summaries_for(passport).await.unwrap();
    assert_eq!(summaries.len(), 2, "only this passport's records");
    assert_eq!(summaries[0], HeldSummary::Erased(receipt));
    let HeldSummary::Present { record, .. } = &summaries[1] else {
        panic!("the record stored second is still present");
    };
    assert_eq!(record, &last);

    let theirs = port.summaries_for(elsewhere).await.unwrap();
    let [HeldSummary::Present { record, .. }] = theirs.as_slice() else {
        panic!("the other passport's one record is listed alone");
    };
    assert_eq!(record, &other);
}

/// A holder that answers the listing itself is asked for it, through the trait
/// object the rest of the crate holds.
#[tokio::test]
async fn a_holder_that_overrides_the_listing_is_asked_for_it() {
    use chrono::{TimeZone, Utc};

    let answer = HeldSummary::Present {
        record: PersonalDataRecordId::new("canned"),
        passport_id: PassportId::new(),
        field: FIELD.to_owned(),
        stored_at: Utc.with_ymd_and_hms(2026, 11, 25, 9, 0, 0).unwrap(),
    };
    let holder = AnswersItself(answer.clone());
    let port: &dyn PersonalDataPort = &holder;
    assert_eq!(
        port.summaries_for(PassportId::new()).await.unwrap(),
        [answer]
    );
}
