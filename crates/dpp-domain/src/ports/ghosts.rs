//! No-op ("ghost") port implementations for development and pre-integration use.
//!
//! Each port whose real adapter depends on an external system not yet
//! available at compile time (object storage, the unpublished EU Central
//! Registry API, a QTSP) ships a synthetic implementation here so calling
//! code compiles and runs against a stable contract before the real
//! integration lands. Grouped together because they share one audience —
//! callers wiring a development or standalone deployment — distinct from the
//! port types/trait files, which are addressed to implementers.
//!
//! Private module: each type is re-exported at its own port's module path
//! (`ports::archive::GhostArchive`, `ports::registry_sync::GhostRegistrySync`,
//! `ports::seal::GhostSeal`) and from the crate root, which is the only
//! public way to reach them.
//!
//! **Deviation, accepted:** the pack's `test-doubles` feature (gating these
//! three types behind `#[cfg(feature = "test-doubles")]` so they cannot ship
//! in a production build) was not implemented. These ghosts always compile
//! in; a caller who wires one into a production deployment gets no
//! compile-time signal. The runtime honesty guard (each ghost's `placeholder:
//! true` / `Pending` / synthetic-ID markers) is the sole safeguard. Accepted
//! because a single always-public path per port is simpler to consume and to
//! reason about than a feature-gated one, and the guard is load-bearing
//! either way; revisit only if a ghost is ever caught reaching production
//! silently.

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use super::archive::{ArchivePort, ArchiveReceipt, ArchiveStatus, ArchiveVerification};
use super::registry_sync::{
    RegistrationRequest, RegistryIdentifiers, RegistryRecord, RegistryStatus, RegistrySyncPort,
};
use super::seal::{
    SealCapabilities, SealFormat, SealMode, SealPort, SealRequest, SealVerification, SealedEnvelope,
};
use crate::domain::error::DppError;
use crate::domain::passport::{Passport, PassportId};

// ─── Archive ──────────────────────────────────────────────────────────────

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

// ─── Registry sync ────────────────────────────────────────────────────────

/// No-op implementation for use before the EU Central Registry API is published.
///
/// Returns synthetic records with `RegistryStatus::Pending` and placeholder
/// identifiers. All operations succeed but perform no real network calls.
pub struct GhostRegistrySync;

#[async_trait]
impl RegistrySyncPort for GhostRegistrySync {
    async fn register(&self, request: RegistrationRequest) -> Result<RegistryRecord, DppError> {
        let now = Utc::now();
        Ok(RegistryRecord {
            identifiers: RegistryIdentifiers {
                product_id: format!("GHOST-PROD-{}", request.passport_id),
                operator_id: format!("GHOST-OP-{}", &request.operator_identifier),
                facility_id: format!("GHOST-FAC-{}", &request.facility_identifier),
                registry_id: format!("GHOST-REG-{}", Uuid::now_v7()),
            },
            status: RegistryStatus::Pending,
            registered_at: now,
            updated_at: now,
        })
    }

    async fn check_status(&self, passport_id: PassportId) -> Result<RegistryRecord, DppError> {
        Err(DppError::NotFound(format!(
            "ghost registry has no record for {passport_id}"
        )))
    }

    async fn notify_transfer(
        &self,
        passport_id: PassportId,
        _new_operator_identifier: String,
    ) -> Result<RegistryRecord, DppError> {
        Err(DppError::NotFound(format!(
            "ghost registry has no record for {passport_id}"
        )))
    }
}

// ─── Seal ─────────────────────────────────────────────────────────────────

/// No-op implementation for use before a QTSP integration is configured.
///
/// Returns synthetic envelopes marked `placeholder: true`. All operations
/// succeed but perform no network I/O and carry no legal validity.
pub struct GhostSeal;

#[async_trait]
impl SealPort for GhostSeal {
    async fn seal(&self, req: SealRequest) -> Result<SealedEnvelope, DppError> {
        Ok(SealedEnvelope {
            format: req.sig_format,
            seal_value: format!(
                "GHOST-SEAL-{}",
                &req.payload_hash[..8.min(req.payload_hash.len())]
            ),
            signing_cert_ref: None,
            sealed_at: Utc::now(),
            placeholder: true,
        })
    }

    async fn verify(&self, env: &SealedEnvelope) -> Result<SealVerification, DppError> {
        Ok(SealVerification {
            valid: false,
            placeholder: env.placeholder,
        })
    }

    fn capabilities(&self) -> SealCapabilities {
        SealCapabilities {
            supported_formats: vec![SealFormat::Jades],
            supported_modes: vec![SealMode::ProviderSeal, SealMode::OperatorSeal],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ghost_register_returns_pending() {
        let sync = GhostRegistrySync;
        let request = RegistrationRequest {
            passport_id: PassportId::new(),
            operator_identifier: "did:web:acme.example.com".into(),
            facility_identifier: "FAC-001".into(),
            facility: None,
            product_category: "textile".into(),
            data_carrier_uri: "https://id.example.com/01/09506000134352".into(),
            schema_version: "1.0.0".into(),
            jws_signature: None,
            published_at: None,
            country_code: String::new(),
        };
        let record = sync.register(request).await.unwrap();
        assert_eq!(record.status, RegistryStatus::Pending);
        assert!(record.identifiers.product_id.starts_with("GHOST-PROD-"));
    }

    #[tokio::test]
    async fn ghost_check_status_returns_not_found() {
        let sync = GhostRegistrySync;
        let result = sync.check_status(PassportId::new()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn ghost_notify_transfer_returns_not_found() {
        let sync = GhostRegistrySync;
        let result = sync
            .notify_transfer(PassportId::new(), "did:web:new-operator.example.com".into())
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn ghost_seal_returns_placeholder() {
        let ghost = GhostSeal;
        let req = SealRequest {
            payload_hash: "abc123def456".into(),
            mode: SealMode::ProviderSeal,
            key_ref: super::super::seal::SealCredentialRef {
                qtsp_id: "test-qtsp".into(),
                credential_id: "cred-001".into(),
            },
            sig_format: SealFormat::Jades,
        };
        let env = ghost.seal(req).await.unwrap();
        assert!(env.placeholder);
        assert!(env.seal_value.starts_with("GHOST-SEAL-"));
        assert_eq!(env.format, SealFormat::Jades);
    }

    #[tokio::test]
    async fn ghost_verify_returns_invalid_placeholder() {
        let ghost = GhostSeal;
        let env = SealedEnvelope {
            format: SealFormat::Jades,
            seal_value: "GHOST-SEAL-abc123".into(),
            signing_cert_ref: None,
            sealed_at: Utc::now(),
            placeholder: true,
        };
        let result = ghost.verify(&env).await.unwrap();
        assert!(!result.valid);
        assert!(result.placeholder);
    }

    #[tokio::test]
    async fn ghost_capabilities_include_jades_and_both_modes() {
        let caps = GhostSeal.capabilities();
        assert!(caps.supported_formats.contains(&SealFormat::Jades));
        assert!(caps.supported_modes.contains(&SealMode::ProviderSeal));
        assert!(caps.supported_modes.contains(&SealMode::OperatorSeal));
    }
}
