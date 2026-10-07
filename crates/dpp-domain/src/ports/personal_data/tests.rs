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
