//! Behaviour of the in-memory archive against the port contract.

use super::stub::InMemoryArchive;
use super::*;
use crate::error::DppError;
use crate::passport::PassportId;
use crate::ports::backup::BackupCopyPort;
use crate::ports::backup::stub::InMemoryBackup;
use chrono::{DateTime, Duration, TimeZone, Utc};
use serde_json::json;
use sha2::{Digest, Sha256};

/// A fixed instant, `minutes` after an arbitrary start.
fn at(minutes: i64) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2027, 3, 1, 12, 0, 0).unwrap() + Duration::minutes(minutes)
}

fn doc(label: &str) -> serde_json::Value {
    json!({ "productName": label })
}

#[tokio::test]
async fn versions_come_back_oldest_first_with_their_documents() {
    let archive = InMemoryArchive::new();
    let id = PassportId::new();
    for (label, minute) in [("first", 10), ("second", 20), ("third", 30)] {
        archive.archive(id, &doc(label), at(minute)).await.unwrap();
    }

    let versions = archive.versions(id).await.unwrap();
    let held: Vec<(&str, DateTime<Utc>)> = versions
        .iter()
        .map(|v| (v.doc["productName"].as_str().unwrap(), v.superseded_at))
        .collect();
    assert_eq!(
        held,
        [("first", at(10)), ("second", at(20)), ("third", at(30))]
    );
    assert!(versions.iter().all(|v| v.passport_id == id));
}

/// A passport that never changed has nothing archived, and the port cannot tell
/// that from a passport it has never heard of: both are an empty answer, not an
/// error. One passport's versions are not another's.
#[tokio::test]
async fn a_passport_with_nothing_archived_has_no_versions() {
    let archive = InMemoryArchive::new();
    let other = PassportId::new();
    archive
        .archive(PassportId::new(), &doc("elsewhere"), at(10))
        .await
        .unwrap();

    assert!(archive.versions(other).await.unwrap().is_empty());
    assert!(archive.version_at(other, at(5)).await.unwrap().is_none());
}

/// Each version is current from the moment its predecessor was replaced until its
/// own `superseded_at`, half-open: at that instant the next one is current.
#[tokio::test]
async fn version_at_is_half_open_on_superseded_at() {
    let archive = InMemoryArchive::new();
    let id = PassportId::new();
    for (label, minute) in [("first", 10), ("second", 20), ("third", 30)] {
        archive.archive(id, &doc(label), at(minute)).await.unwrap();
    }

    let current = |minute: i64| {
        let archive = &archive;
        async move {
            archive
                .version_at(id, at(minute))
                .await
                .unwrap()
                .map(|v| v.doc["productName"].as_str().unwrap().to_owned())
        }
    };
    // Before the first change the port still answers with the first version: it
    // does not know when the passport was created.
    assert_eq!(current(5).await.as_deref(), Some("first"));
    assert_eq!(current(10).await.as_deref(), Some("second"));
    assert_eq!(current(15).await.as_deref(), Some("second"));
    assert_eq!(current(20).await.as_deref(), Some("third"));
    assert_eq!(current(29).await.as_deref(), Some("third"));
    // After the last archived change the live record is the answer.
    assert_eq!(current(30).await, None);
    assert_eq!(current(99).await, None);
}

#[tokio::test]
async fn a_version_that_would_precede_the_latest_is_refused_and_nothing_is_kept() {
    let archive = InMemoryArchive::new();
    let id = PassportId::new();
    archive.archive(id, &doc("later"), at(20)).await.unwrap();

    let refused = archive.archive(id, &doc("earlier"), at(10)).await;
    assert!(
        matches!(refused, Err(DppError::Validation(_))),
        "{refused:?}"
    );
    assert_eq!(archive.versions(id).await.unwrap().len(), 1);
}

#[tokio::test]
async fn one_instant_cannot_end_two_different_versions() {
    let archive = InMemoryArchive::new();
    let id = PassportId::new();
    archive.archive(id, &doc("held"), at(10)).await.unwrap();

    let refused = archive.archive(id, &doc("rival"), at(10)).await;
    assert!(
        matches!(refused, Err(DppError::Validation(_))),
        "{refused:?}"
    );
    let versions = archive.versions(id).await.unwrap();
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].doc, doc("held"));
}

/// A caller that lost the answer can ask again, even after later versions have
/// been archived, and gets the original receipt back.
#[tokio::test]
async fn a_retry_returns_the_original_receipt_and_keeps_nothing_new() {
    let archive = InMemoryArchive::new();
    let id = PassportId::new();
    let original = archive.archive(id, &doc("first"), at(10)).await.unwrap();
    archive.archive(id, &doc("second"), at(20)).await.unwrap();

    let retried = archive.archive(id, &doc("first"), at(10)).await.unwrap();
    assert_eq!(retried, original);
    assert_eq!(archive.versions(id).await.unwrap().len(), 2);
}

/// The hash is SHA-256 over the RFC 8785 form. The expectation is the canonical
/// bytes written out by hand, not this crate's own canonicaliser, so the test
/// cannot agree with a wrong implementation. `serde_json` already sorts keys, so
/// the number is what tells the two apart: RFC 8785 writes `1.0` as `1`, and
/// plain `serde_json` writes it as `1.0`.
#[tokio::test]
async fn the_hash_is_the_sha256_of_the_rfc_8785_form() {
    let archive = InMemoryArchive::new();
    let id = PassportId::new();
    let unordered: serde_json::Value = serde_json::from_str(r#"{"b": 1.0, "a": [2, 3]}"#).unwrap();
    let expected = hex::encode(Sha256::digest(br#"{"a":[2,3],"b":1}"#));

    let receipt = archive.archive(id, &unordered, at(10)).await.unwrap();

    assert_eq!(receipt.content_hash, expected);
    assert_eq!(
        archive.versions(id).await.unwrap()[0].content_hash,
        expected
    );
}

/// An archive is evidence, so what comes back is what went in, including what no
/// struct in this crate knows about.
#[tokio::test]
async fn the_document_is_kept_as_written() {
    let archive = InMemoryArchive::new();
    let id = PassportId::new();
    let written = json!({
        "productName": "Test",
        "aFieldNoStructKnows": { "nested": [1, null, "x"] },
    });

    archive.archive(id, &written, at(10)).await.unwrap();

    assert_eq!(archive.versions(id).await.unwrap()[0].doc, written);
}

/// One canonicalisation serves both ports: the back-up copy of a passport and the
/// archived version of that same document carry the same hash.
#[tokio::test]
async fn a_back_up_copy_and_an_archived_version_of_one_document_share_a_hash() {
    let passport = crate::test_support::sample_passport();
    let backed_up = InMemoryBackup::new().store(&passport, 10).await.unwrap();

    let archived = InMemoryArchive::new()
        .archive(
            passport.id,
            &serde_json::to_value(&passport).unwrap(),
            at(10),
        )
        .await
        .unwrap();

    assert_eq!(archived.content_hash, backed_up.content_hash);
}
