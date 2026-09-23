//! The carrier serial: what `validate` admits, and what the effective value is
//! when the operator has attributed nothing.

use super::tests::make_passport;

#[test]
fn an_unattributed_passport_carries_the_default_derived_from_its_id() {
    let p = make_passport();
    assert_eq!(p.carrier_serial, None);
    assert_eq!(p.effective_carrier_serial(), p.id.default_carrier_serial());
}

/// An operator that already serialises its units attributes that serial, and
/// the label then carries it rather than the default.
#[test]
fn an_attributed_serial_is_the_effective_one() {
    let mut p = make_passport();
    p.carrier_serial = Some("SN-2026/00042".to_owned());
    assert!(p.validate().is_ok());
    assert_eq!(p.effective_carrier_serial(), "SN-2026/00042");
}

/// The manufacturer's serial and the carrier serial are separate facts, and
/// recording the first does not attribute the second.
#[test]
fn a_manufacturer_serial_does_not_become_the_carrier_serial() {
    let mut p = make_passport();
    p.serial_number = Some("SN-2026-00042".to_owned());
    assert_eq!(p.effective_carrier_serial(), p.id.default_carrier_serial());
}

/// Every way an attributed value can fail GS1 AI 21 is refused at the field
/// the carrier is built from, before any label can be printed from it.
#[test]
fn validate_refuses_a_serial_gs1_would_reject() {
    for (bad, expected) in [
        ("", "must not be empty"),
        ("123456789012345678901", "has 21 characters"),
        ("SN 1", "outside GS1 CSET 82"),
        ("SN#1", "outside GS1 CSET 82"),
        ("SNé1", "outside GS1 CSET 82"),
    ] {
        let mut p = make_passport();
        p.carrier_serial = Some(bad.to_owned());
        let err = p.validate().expect_err(bad).to_string();
        assert!(
            err.contains("carrier_serial") && err.contains(expected),
            "{bad:?}: {err}"
        );
    }
}

/// Additive on the wire: a document written before the field existed reads
/// back with no attributed serial, and so keeps the default it always had.
#[test]
fn a_document_without_the_key_reads_back_with_the_default() {
    let p = make_passport();
    let json = serde_json::to_value(&p).expect("serialises");
    assert!(
        json.get("carrierSerial").is_none(),
        "None is omitted: {json}"
    );

    let back: super::Passport = serde_json::from_value(json).expect("reads back");
    assert_eq!(back.carrier_serial, None);
    assert_eq!(
        back.effective_carrier_serial(),
        p.id.default_carrier_serial()
    );
}
