//! The personal-data port's contract, held to by its in-memory implementation.

use serde_json::json;

use super::stub::InMemoryPersonalData;
use super::{HeldRecord, PersonalDataPort};
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
