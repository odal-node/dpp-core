//! Behaviour of the in-memory archive against the port contract.

use super::stub::InMemoryArchive;
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
            did_web_url: Some("https://test.example.com/.well-known/did.json".into()),
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
async fn archive_and_retrieve() {
    let archive = InMemoryArchive::new();
    let passport = make_test_passport();
    let receipt = archive.archive(&passport, 10).await.unwrap();
    assert!(!receipt.content_hash.is_empty());
    assert!(receipt.archive_id.starts_with("ARCHIVE-"));

    let retrieved = archive.retrieve(passport.id).await.unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().id, passport.id);
}

#[tokio::test]
async fn verify_integrity_ok() {
    let archive = InMemoryArchive::new();
    let passport = make_test_passport();
    let receipt = archive.archive(&passport, 10).await.unwrap();

    let verification = archive
        .verify(passport.id, &receipt.content_hash)
        .await
        .unwrap();
    assert!(verification.integrity_ok);
    assert!(verification.accessible);
    assert_eq!(verification.status, ArchiveStatus::Active);
}

#[tokio::test]
async fn verify_integrity_mismatch() {
    let archive = InMemoryArchive::new();
    let passport = make_test_passport();
    archive.archive(&passport, 10).await.unwrap();

    let verification = archive.verify(passport.id, "bad-hash").await.unwrap();
    assert!(!verification.integrity_ok);
}

#[tokio::test]
async fn update_archive_changes_hash() {
    let archive = InMemoryArchive::new();
    let mut passport = make_test_passport();
    let receipt1 = archive.archive(&passport, 10).await.unwrap();

    passport.product_name = "Updated Textile".into();
    let receipt2 = archive.update_archive(&passport).await.unwrap();
    assert_ne!(receipt1.content_hash, receipt2.content_hash);
}

#[tokio::test]
async fn retrieve_nonexistent_returns_none() {
    let archive = InMemoryArchive::new();
    let result = archive.retrieve(PassportId::new()).await.unwrap();
    assert!(result.is_none());
}
