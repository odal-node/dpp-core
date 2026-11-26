//! The optional commodity code and back-up URL of a payload.

use super::super::*;
use super::common::*;

/// Absent is lawful: the regulation qualifies the commodity code "where
/// relevant", and a product group that does not call for one must still register.
#[test]
fn a_payload_without_a_commodity_code_validates() {
    let mut payload = sample_payload();
    payload.commodity_code = None;
    assert!(payload.validate().is_ok());
}

/// Structurally malformed is not. Whether the code is the *right* one for the
/// product group is the registry's check against ranges we do not hold; whether
/// it is a tariff code at all is checkable here.
#[test]
fn a_malformed_commodity_code_is_refused() {
    for bad in ["8507", "8507 60 00", "notacode", "850760009"] {
        let mut payload = sample_payload();
        payload.commodity_code = Some(bad.into());
        assert!(
            matches!(
                payload.validate(),
                Err(RegistryValidationError::InvalidCommodityCode { .. })
            ),
            "{bad} must be refused"
        );
    }
}

#[test]
fn the_three_tariff_levels_all_validate() {
    for good in ["850760", "85076000", "8507600090"] {
        let mut payload = sample_payload();
        payload.commodity_code = Some(good.into());
        assert!(payload.validate().is_ok(), "{good} must validate");
    }
}

/// A back-up the registry cannot fetch over TLS is worse than none declared.
#[test]
fn an_insecure_backup_url_is_refused() {
    let mut payload = sample_payload();
    payload.backup_url = Some("http://backup.example.com/dpp/abc.json".into());
    assert!(matches!(
        payload.validate(),
        Err(RegistryValidationError::InsecureBackupUrl { .. })
    ));
}

/// Declaring no back-up is lawful — storing snapshots is not the same as
/// publishing them, and a deployment that does not publish one says so.
#[test]
fn a_payload_without_a_backup_url_validates() {
    let mut payload = sample_payload();
    payload.backup_url = None;
    assert!(payload.validate().is_ok());
}

/// Both travel on the wire, and both are omitted rather than nulled when unset.
#[test]
fn commodity_code_and_backup_url_serialise() {
    let payload = sample_payload();
    let json = serde_json::to_value(&payload).unwrap();
    assert_eq!(json["commodityCode"], "85076000");
    assert_eq!(json["backupUrl"], "https://backup.example.com/dpp/abc.json");

    let mut bare = sample_payload();
    bare.commodity_code = None;
    bare.backup_url = None;
    let json = serde_json::to_value(&bare).unwrap();
    assert!(json.get("commodityCode").is_none());
    assert!(json.get("backupUrl").is_none());
}

// ── The observed contract ────────────────────────────────────────────────────
//
// Each of these pins something read from the registry's own published material
// on 2026-09-16 — its web client, or the User Guide v1.02. None of it is a
// specification, so the tests exist to make a later correction *visible* rather
// than to assert conformance.
