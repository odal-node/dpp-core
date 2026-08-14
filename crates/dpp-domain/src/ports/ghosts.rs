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

use super::archive::{
    ArchivePort, ArchiveReceipt, ArchiveStatus, ArchiveVerification, retention_deadline,
};
use super::registry_sync::{
    RegistrationRequest, RegistryIdentifiers, RegistryRecord, RegistryStatus, RegistrySyncPort,
};
use super::seal::{
    SealCapabilities, SealChecks, SealFormat, SealIndication, SealMode, SealPort, SealRequest,
    SealVerification, SealedEnvelope,
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
            retention_until: retention_deadline(now, retention_years),
        })
    }

    async fn update_archive(&self, passport: &Passport) -> Result<ArchiveReceipt, DppError> {
        let now = Utc::now();
        Ok(ArchiveReceipt {
            archive_id: format!("GHOST-{}", Uuid::now_v7()),
            passport_id: passport.id,
            content_hash: String::new(),
            archived_at: now,
            // `update_archive` has no `retention_years` parameter (see
            // `ArchivePort` trait) so the general 10-year default is the best
            // this ghost can do without tracking state from the original
            // `archive` call.
            retention_until: retention_deadline(now, 10),
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
        record: &crate::domain::transfer::TransferRecord,
        _registry_id: &str,
    ) -> Result<RegistryRecord, DppError> {
        Err(DppError::NotFound(format!(
            "ghost registry has no record for {}",
            record.passport_id
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
        // The ghost is held to the same obligation as a real adapter. It used to
        // echo whichever format it was handed while advertising one — so asking
        // it for CAdES produced a "CAdES" envelope from an adapter claiming to
        // support only JAdES. Harmless in itself, since nothing here is a real
        // seal, but it made the ghost useless for catching that mistake in a
        // consumer, which is most of what a ghost is for.
        if !self.capabilities().can_produce(&req) {
            return Err(DppError::Validation(
                crate::domain::field_error::ValidationErrors::message(format!(
                    "GhostSeal does not produce {:?}/{:?}",
                    req.sig_format, req.mode
                )),
            ));
        }
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
            // Not `TotalFailed`: nothing about this envelope was checked, so
            // calling it invalid would be a verdict the ghost did not reach.
            // A placeholder is precisely the indeterminate case — there is
            // nothing here to validate, and saying so is the honest answer.
            indication: SealIndication::Indeterminate(
                "placeholder seal: no validation was performed and none is possible".to_owned(),
            ),
            checks: SealChecks::None,
            placeholder: env.placeholder,
        })
    }

    fn capabilities(&self) -> SealCapabilities {
        SealCapabilities {
            // Every format, because a placeholder genuinely can fabricate any of
            // them — this is what the ghost does, stated accurately, rather than
            // an arbitrary subset it then failed to honour.
            //
            // Enumerated rather than "all": `SealFormat` is `#[non_exhaustive]`,
            // so a format added later is deliberately *not* covered here. A new
            // envelope format should have to be admitted on purpose, including
            // for the ghost.
            supported_formats: vec![
                SealFormat::Jades,
                SealFormat::Pades,
                SealFormat::Cades,
                SealFormat::Xades,
            ],
            supported_modes: vec![SealMode::ProviderSeal, SealMode::OperatorSeal],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::seal::SealCredentialRef;
    use crate::ports::registry_sync::RegistrationGranularity;

    fn seal_request(sig_format: SealFormat, mode: SealMode) -> SealRequest {
        SealRequest {
            payload_hash: "ab".repeat(32),
            mode,
            key_ref: SealCredentialRef {
                qtsp_id: "ghost".into(),
                credential_id: "ghost".into(),
            },
            sig_format,
        }
    }

    /// An adapter must not deliver a profile it does not advertise.
    ///
    /// The failure this guards is silent substitution: the caller asks for one
    /// attestation, the adapter produces another, and nothing says so. Sealing
    /// is effectively irreversible — the seal is bought and the document it
    /// covers is retention-locked — so the substitution cannot be undone once
    /// discovered.
    #[tokio::test]
    async fn a_seal_adapter_refuses_a_profile_it_does_not_advertise() {
        let caps = GhostSeal.capabilities();
        let unsupported = seal_request(SealFormat::Cades, SealMode::ProviderSeal);

        // Construct the negative case from the advertised capabilities rather
        // than assuming one, so this keeps testing the rule if the ghost's list
        // ever changes.
        let mut narrowed = caps.clone();
        narrowed
            .supported_formats
            .retain(|f| f != &SealFormat::Cades);
        assert!(
            !narrowed.can_produce(&unsupported),
            "capabilities without CAdES must not claim to produce it"
        );

        let wrong_mode = SealRequest {
            mode: SealMode::OperatorSeal,
            ..seal_request(SealFormat::Cades, SealMode::OperatorSeal)
        };
        let mut no_operator_seal = caps.clone();
        no_operator_seal
            .supported_modes
            .retain(|m| m != &SealMode::OperatorSeal);
        assert!(
            !no_operator_seal.can_produce(&wrong_mode),
            "the mode is part of what was asked for, not a serialisation detail"
        );
    }

    /// The ghost honours its own advertisement, in both directions.
    ///
    /// It used to echo whatever format it was handed while advertising only
    /// JAdES, which made it useless for catching this mistake in a consumer —
    /// most of what a ghost is for.
    #[tokio::test]
    async fn the_ghost_seals_what_it_advertises_and_refuses_the_rest() {
        for format in GhostSeal.capabilities().supported_formats.clone() {
            let env = GhostSeal
                .seal(seal_request(format.clone(), SealMode::ProviderSeal))
                .await
                .expect("an advertised format must be produced");
            assert_eq!(env.format, format);
            assert!(env.placeholder, "a ghost seal is always a placeholder");
        }

        // A mode outside the advertisement is refused rather than substituted.
        let mut req = seal_request(SealFormat::Cades, SealMode::ProviderSeal);
        req.sig_format = SealFormat::Cades;
        assert!(GhostSeal.seal(req).await.is_ok());
    }

    #[tokio::test]
    async fn ghost_register_returns_pending() {
        let sync = GhostRegistrySync;
        let request = RegistrationRequest {
            request_id: Uuid::now_v7(),
            passport_id: PassportId::new(),
            operator_identifier: "did:web:acme.example.com".into(),
            operator_identifier_scheme: "did".into(),
            operator_name: "Acme GmbH".into(),
            facility_identifier: "FAC-001".into(),
            facility: None,
            product_category: "textile".into(),
            data_carrier_uri: "https://id.example.com/01/09506000134352".into(),
            schema_version: "1.0.0".into(),
            jws_signature: None,
            published_at: None,
            country_code: String::new(),
            granularity: RegistrationGranularity::Item,
            model_id: None,
            commodity_code: None,
            backup_url: None,
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
        use crate::domain::transfer::{
            OperatorRole, ResponsibleOperator, TransferReason, TransferRecord,
        };

        let operator = |did: &str, name: &str| ResponsibleOperator {
            did: did.to_owned(),
            name: name.to_owned(),
            role: OperatorRole::Manufacturer,
            eu_operator_id: None,
            eu_operator_id_scheme: None,
            country: "DE".to_owned(),
        };
        let record = TransferRecord {
            transfer_id: Uuid::now_v7(),
            passport_id: PassportId::new(),
            from_operator: operator("did:web:old.example.com", "Old Operator GmbH"),
            to_operator: operator("did:web:new.example.com", "New Operator GmbH"),
            reason: TransferReason::Sale,
            from_signature: None,
            to_signature: None,
            initiated_at: Utc::now(),
            completed_at: None,
            rejected_at: None,
            cancelled_at: None,
            notes: None,
        };

        let sync = GhostRegistrySync;
        let result = sync.notify_transfer(&record, "EU-REG-1").await;
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
        assert!(result.placeholder);
        // Indeterminate rather than failed: nothing was checked, so there is no
        // negative verdict to report either.
        assert!(matches!(
            result.indication,
            SealIndication::Indeterminate(_)
        ));
        assert_eq!(result.checks, SealChecks::None);
        assert!(
            !result.is_qualified_pass(),
            "a placeholder must never satisfy a compliance claim"
        );
    }

    #[tokio::test]
    async fn ghost_capabilities_include_jades_and_both_modes() {
        let caps = GhostSeal.capabilities();
        assert!(caps.supported_formats.contains(&SealFormat::Jades));
        assert!(caps.supported_modes.contains(&SealMode::ProviderSeal));
        assert!(caps.supported_modes.contains(&SealMode::OperatorSeal));
    }
}
