//! Validation of a transfer notification.

use super::super::*;
use super::common::*;
use chrono::Utc;
use uuid::Uuid;

fn sample_transfer() -> TransferNotification {
    TransferNotification {
        passport_id: Uuid::nil(),
        registry_id: "EU-REG-2026-00001".into(),
        from_operator: sample_operator_id(),
        to_operator: OperatorIdentifier {
            scheme: "vat".into(),
            value: "FR987654321".into(),
            name: "ModeVerte SARL".into(),
            country: "FR".into(),
            did: Some("did:web:modeverte.fr".into()),
        },
        reason: "sale".into(),
        transferred_at: Utc::now(),
        from_signature: Some("sig_from...".into()),
        node_acceptance_attestation: Some("sig_to...".into()),
    }
}

#[test]
fn valid_transfer_notification_passes() {
    assert!(sample_transfer().validate().is_ok());
}

/// A transfer names the two legal persons on either side of the handover, so an
/// unidentified operator on *either* side must be refused. This is the check
/// whose absence let an adapter send empty strings for both.
#[test]
fn transfer_with_an_unidentified_operator_is_refused() {
    let mut from_blank = sample_transfer();
    from_blank.from_operator.name = String::new();
    assert!(
        matches!(
            from_blank.validate(),
            Err(RegistryValidationError::MissingRequiredField(f)) if f == "operatorId.name"
        ),
        "the outgoing operator must be identified"
    );

    let mut to_blank = sample_transfer();
    to_blank.to_operator.name = String::new();
    assert!(
        matches!(
            to_blank.validate(),
            Err(RegistryValidationError::MissingRequiredField(f)) if f == "operatorId.name"
        ),
        "the incoming operator must be identified"
    );
}

#[test]
fn transfer_with_an_invalid_country_is_refused() {
    let mut notif = sample_transfer();
    notif.to_operator.country = "XX".into();
    assert!(matches!(
        notif.validate(),
        Err(RegistryValidationError::InvalidCountryCode { .. })
    ));
}

#[test]
fn transfer_without_a_reason_is_refused() {
    let mut notif = sample_transfer();
    notif.reason = "   ".into();
    assert!(matches!(
        notif.validate(),
        Err(RegistryValidationError::MissingRequiredField(f)) if f == "reason"
    ));
}

/// A transfer is initiated by the outgoing operator and countersigned only when
/// the incoming one accepts, so a pending transfer legitimately has no
/// `node_acceptance_attestation`. Requiring it here would make the notification unbuildable
/// for exactly the case a registry most wants to hear about.
#[test]
fn a_pending_transfer_still_validates() {
    let mut notif = sample_transfer();
    notif.node_acceptance_attestation = None;
    assert!(notif.validate().is_ok());
}

/// An identifier with no scheme is not identifiable: the value alone does not
/// say whether it is a VAT number, an LEI or a DID. This is the check that
/// stops an unscheme'd identifier being submitted as if it were well-formed —
/// the per-scheme check accepts any unrecognised scheme, including the empty
/// one, so without this it would pass.
#[test]
fn operator_identifier_without_a_scheme_is_refused() {
    for scheme in ["", "   "] {
        let mut oid = sample_operator_id();
        oid.scheme = scheme.into();
        assert!(
            matches!(
                oid.validate(),
                Err(RegistryValidationError::MissingRequiredField(f)) if f == "operatorId.scheme"
            ),
            "an identifier with scheme {scheme:?} must be refused"
        );
    }
}

// ── Commodity code and back-up link ─────────────────────────────────────────
