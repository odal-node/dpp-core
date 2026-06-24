//! Port trait for ESPR-mandated third-party DPP archival.
//!
//! EU ESPR requires that DPP data remains accessible for the period defined
//! in the applicable delegated act, even in cases of insolvency or market
//! withdrawal by the economic operator. A copy of the DPP must be hosted by
//! an independent third-party digital service provider.
//!
//! This port defines the contract that platform adapters implement to
//! replicate published passport data to an independent archive.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::{
    error::DppError,
    passport::{Passport, PassportId},
};

// ─── Types ───────────────────────────────────────────────────────────────

/// Confirmation receipt from the third-party archive.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveReceipt {
    /// Archive-assigned identifier for this stored copy.
    pub archive_id: String,
    /// The passport ID of the archived record.
    pub passport_id: PassportId,
    /// Cryptographic hash (SHA-256) of the archived payload for integrity verification.
    pub content_hash: String,
    /// Timestamp when the archive accepted the record.
    pub archived_at: DateTime<Utc>,
    /// The retention period end date (derived from the applicable delegated act).
    /// The archive MUST retain the record until at least this date.
    pub retention_until: DateTime<Utc>,
}

/// Status of a passport record within the third-party archive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum ArchiveStatus {
    /// Record is stored and accessible.
    Active,
    /// Record has been updated (e.g. after a transfer of responsibility).
    Updated,
    /// Record is within the retention-locked period and cannot be removed.
    RetentionLocked,
    /// Retention period has expired; record may be purged by the archive.
    Expired,
}

/// Verification result from the archive integrity check.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveVerification {
    /// Whether the archived copy matches the provided content hash.
    pub integrity_ok: bool,
    /// Whether the archive confirms the record is still accessible.
    pub accessible: bool,
    /// Current archive status.
    pub status: ArchiveStatus,
    /// Timestamp of the last integrity check performed by the archive.
    pub last_verified_at: DateTime<Utc>,
}

// ─── Port Trait ──────────────────────────────────────────────────────────

/// Port trait for replicating DPP records to an independent third-party archive.
///
/// Called automatically when a passport is published. Platform adapters
/// implement this trait to connect to the chosen archive service provider.
///
/// # SLA expectations
///
/// The archive provider MUST:
/// - Accept and store the record within the SLA window (recommended < 30s).
/// - Return a content hash for integrity verification.
/// - Retain the record for the full retention period.
/// - Serve the record upon authenticated request even if the originating
///   operator's infrastructure is unreachable (insolvency failover).
#[async_trait]
pub trait ArchivePort: Send + Sync {
    /// Archive a published passport.
    ///
    /// Called on the `Draft → Published` transition. The passport's JWS
    /// signature MUST be present (i.e. the passport has been signed).
    ///
    /// `retention_years` is derived from the applicable delegated act
    /// (typically 10–15 years after the product's end of life).
    async fn archive(
        &self,
        passport: &Passport,
        retention_years: u32,
    ) -> Result<ArchiveReceipt, DppError>;

    /// Update an existing archived record.
    ///
    /// Called when a passport is updated after a transfer of responsibility
    /// or when compliance data is corrected. The archive MUST store the
    /// new version while preserving the full version history.
    async fn update_archive(&self, passport: &Passport) -> Result<ArchiveReceipt, DppError>;

    /// Verify that the archive holds an intact copy of the passport.
    ///
    /// Compares a content hash against the archived payload. Used for
    /// periodic integrity audits and compliance verification.
    async fn verify(
        &self,
        passport_id: PassportId,
        expected_hash: &str,
    ) -> Result<ArchiveVerification, DppError>;

    /// Retrieve a passport from the archive.
    ///
    /// Used as a failover when the originating operator's infrastructure
    /// is unreachable. Returns `None` if the archive has no record.
    async fn retrieve(&self, passport_id: PassportId) -> Result<Option<Passport>, DppError>;
}

// ─── Ghost implementation (development / pre-config) ─────────────────────────

/// No-op archive for development and standalone vault deployments.
///
/// All operations succeed without performing any I/O. Returns synthetic
/// receipts with `archive_id = "ghost-{uuid}"`. Use in tests and in the
/// standalone `dpp-vault` binary where object storage is not configured.
pub struct GhostArchive;

#[async_trait]
impl ArchivePort for GhostArchive {
    async fn archive(
        &self,
        passport: &Passport,
        retention_years: u32,
    ) -> Result<ArchiveReceipt, DppError> {
        let now = Utc::now();
        Ok(ArchiveReceipt {
            archive_id: format!("GHOST-{}", Uuid::now_v7()),
            passport_id: passport.id,
            content_hash: String::new(),
            archived_at: now,
            retention_until: now + chrono::Duration::days(365 * retention_years as i64),
        })
    }

    async fn update_archive(&self, passport: &Passport) -> Result<ArchiveReceipt, DppError> {
        let now = Utc::now();
        Ok(ArchiveReceipt {
            archive_id: format!("GHOST-{}", Uuid::now_v7()),
            passport_id: passport.id,
            content_hash: String::new(),
            archived_at: now,
            retention_until: now + chrono::Duration::days(365 * 10),
        })
    }

    async fn verify(
        &self,
        _passport_id: PassportId,
        _expected_hash: &str,
    ) -> Result<ArchiveVerification, DppError> {
        Ok(ArchiveVerification {
            integrity_ok: false,
            accessible: false,
            status: ArchiveStatus::Expired,
            last_verified_at: Utc::now(),
        })
    }

    async fn retrieve(&self, _passport_id: PassportId) -> Result<Option<Passport>, DppError> {
        Ok(None)
    }
}

// ─── In-memory stub (testing) ────────────────────────────────────────────

/// In-memory archive implementation for testing.
///
/// Stores passports in a `HashMap` behind a `Mutex`. Not suitable for
/// production — use a real archive service adapter.
#[cfg(any(test, feature = "test-utils"))]
pub mod stub {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::collections::HashMap;
    use std::sync::Mutex;

    pub struct InMemoryArchive {
        store: Mutex<HashMap<PassportId, (Passport, ArchiveReceipt)>>,
    }

    impl InMemoryArchive {
        pub fn new() -> Self {
            Self {
                store: Mutex::new(HashMap::new()),
            }
        }

        fn hash_passport(passport: &Passport) -> String {
            let json = serde_json::to_vec(passport).unwrap_or_default();
            let digest = Sha256::digest(&json);
            hex::encode(digest)
        }
    }

    impl Default for InMemoryArchive {
        fn default() -> Self {
            Self::new()
        }
    }

    #[async_trait]
    impl ArchivePort for InMemoryArchive {
        async fn archive(
            &self,
            passport: &Passport,
            retention_years: u32,
        ) -> Result<ArchiveReceipt, DppError> {
            let now = Utc::now();
            let retention_until = now + chrono::Duration::days(365 * retention_years as i64);
            let hash = Self::hash_passport(passport);
            let receipt = ArchiveReceipt {
                archive_id: format!("ARCHIVE-{}", uuid::Uuid::now_v7()),
                passport_id: passport.id,
                content_hash: hash,
                archived_at: now,
                retention_until,
            };
            let mut store = self.store.lock().unwrap();
            store.insert(passport.id, (passport.clone(), receipt.clone()));
            Ok(receipt)
        }

        async fn update_archive(&self, passport: &Passport) -> Result<ArchiveReceipt, DppError> {
            let mut store = self.store.lock().unwrap();
            if let Some((stored, receipt)) = store.get_mut(&passport.id) {
                *stored = passport.clone();
                receipt.content_hash = Self::hash_passport(passport);
                Ok(receipt.clone())
            } else {
                Err(DppError::NotFound(format!(
                    "no archived record for {}",
                    passport.id
                )))
            }
        }

        async fn verify(
            &self,
            passport_id: PassportId,
            expected_hash: &str,
        ) -> Result<ArchiveVerification, DppError> {
            let store = self.store.lock().unwrap();
            if let Some((_, receipt)) = store.get(&passport_id) {
                Ok(ArchiveVerification {
                    integrity_ok: receipt.content_hash == expected_hash,
                    accessible: true,
                    status: ArchiveStatus::Active,
                    last_verified_at: Utc::now(),
                })
            } else {
                Err(DppError::NotFound(format!(
                    "no archived record for {passport_id}"
                )))
            }
        }

        async fn retrieve(&self, passport_id: PassportId) -> Result<Option<Passport>, DppError> {
            let store = self.store.lock().unwrap();
            Ok(store.get(&passport_id).map(|(p, _)| p.clone()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::stub::InMemoryArchive;
    use super::*;
    use crate::domain::passport::*;
    use crate::domain::sector::{CarbonFootprint, RepairabilityScore, Sector};
    use crate::domain::status::PassportStatus;
    use chrono::Utc;

    fn make_test_passport() -> Passport {
        Passport {
            id: PassportId::new(),
            batch_id: None,
            product_name: "Test Textile".into(),
            sector: Sector::Textile,
            product_category: None,
            manufacturer: ManufacturerInfo {
                name: "Test Brand".into(),
                address: "Berlin, DE".into(),
                did_web_url: Some("https://test.example.com/.well-known/did.json".into()),
            },
            materials: vec![],
            co2e_per_unit: Some(CarbonFootprint::from_kg(3.5)),
            repairability_score: Some(RepairabilityScore::from_scalar(7.0)),
            sector_data: None,
            status: PassportStatus::Published,
            qr_code_url: None,
            jws_signature: Some("eyJ0eXAiOiJKV1QifQ.test.signature".into()),
            public_jws_signature: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            published_at: Some(Utc::now()),
            schema_version: "1.0.0".into(),
            retention_locked: true,
            version: 1,
            supersedes_id: None,
            retention_until: None,
            product_id: None,
            operator_identifier: None,
            facility_id: None,
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
}
