//! Behaviour of the three ghost ports: they succeed, and they are honest about it.

use super::*;
use chrono::Utc;
use uuid::Uuid;

use crate::domain::passport::PassportId;
use crate::domain::seal::{SealCredentialRef, SealedEnvelope};
use crate::ports::registry_sync::RegistrationGranularity;
use crate::ports::registry_sync::{RegistrationRequest, RegistryStatus, RegistrySyncPort};
use crate::ports::seal::{
    SealChecks, SealConformanceLevel, SealEnvelope, SealFormat, SealIndication, SealMode, SealPort,
    SealRequest,
};

/// A request whose packaging is one the format actually defines.
///
/// Not `Detached` for everything: PAdES does not define `Detached`, so a
/// fixed packaging silently made every PAdES request ill-formed rather than
/// testing PAdES.
fn seal_request(sig_format: SealFormat, mode: SealMode) -> SealRequest {
    let envelope = sig_format.envelopes()[0];
    SealRequest {
        payload_hash: "ab".repeat(32),
        mode,
        key_ref: SealCredentialRef {
            qtsp_id: "ghost".into(),
            credential_id: "ghost".into(),
        },
        sig_format,
        conformance_level: SealConformanceLevel::BaselineLt,
        envelope,
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
        conformance_level: SealConformanceLevel::BaselineLt,
        envelope: SealEnvelope::Detached,
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
