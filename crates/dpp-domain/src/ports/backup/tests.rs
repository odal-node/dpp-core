//! Behaviour of the in-memory back-up copy against the port contract.

use super::stub::InMemoryBackup;
use super::*;
use crate::passport::*;
use crate::product_group::{CarbonFootprint, RepairabilityScore};
use crate::status::PassportStatus;
use chrono::Utc;

fn make_test_passport() -> Passport {
    Passport {
        product_name: "Test Textile".into(),
        manufacturer: ManufacturerInfo {
            name: "Test Brand".into(),
            address: "Berlin, DE".into(),
            country: None,
            did_web_url: Some("https://test.example.com/.well-known/did.json".into()),
            registered_trade_name: None,
            electronic_address: None,
        },
        co2e_per_unit: Some(CarbonFootprint::from_kg(3.5)),
        repairability_score: Some(RepairabilityScore::from_scalar(7.0)),
        status: PassportStatus::Published,
        jws_signature: Some("eyJ0eXAiOiJKV1QifQ.test.signature".into()),
        published_at: Some(Utc::now()),
        placed_on_market_date: None,
        retention_locked: true,
        ..crate::test_support::sample_passport()
    }
}

#[tokio::test]
async fn backup_and_retrieve() {
    let backup = InMemoryBackup::new();
    let passport = make_test_passport();
    let receipt = backup.store(&passport, 10).await.unwrap();
    assert!(!receipt.content_hash.is_empty());
    assert!(receipt.backup_id.starts_with("BACKUP-"));

    let retrieved = backup.retrieve(passport.id).await.unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().id, passport.id);
}

#[tokio::test]
async fn verify_integrity_ok() {
    let backup = InMemoryBackup::new();
    let passport = make_test_passport();
    let receipt = backup.store(&passport, 10).await.unwrap();

    let verification = backup
        .verify(passport.id, &receipt.content_hash)
        .await
        .unwrap();
    assert!(verification.integrity_ok);
    assert!(verification.accessible);
    assert_eq!(verification.status, BackupStatus::Active);
}

#[tokio::test]
async fn verify_integrity_mismatch() {
    let backup = InMemoryBackup::new();
    let passport = make_test_passport();
    backup.store(&passport, 10).await.unwrap();

    let verification = backup.verify(passport.id, "bad-hash").await.unwrap();
    assert!(!verification.integrity_ok);
}

#[tokio::test]
async fn update_changes_hash() {
    let backup = InMemoryBackup::new();
    let mut passport = make_test_passport();
    let receipt1 = backup.store(&passport, 10).await.unwrap();

    passport.product_name = "Updated Textile".into();
    let receipt2 = backup.update(&passport).await.unwrap();
    assert_ne!(receipt1.content_hash, receipt2.content_hash);
}

#[tokio::test]
async fn retrieve_nonexistent_returns_none() {
    let backup = InMemoryBackup::new();
    let result = backup.retrieve(PassportId::new()).await.unwrap();
    assert!(result.is_none());
}
