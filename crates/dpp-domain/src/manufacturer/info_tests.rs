//! What `ManufacturerInfo::country` accepts, what the Art. 27(6) contact fields
//! carry, and what a document written before any of them existed does.

use super::ManufacturerInfo;

fn manufacturer(country: Option<&str>) -> ManufacturerInfo {
    ManufacturerInfo {
        name: "GreenCell GmbH".to_owned(),
        address: "Alexanderplatz 1, Berlin".to_owned(),
        country: country.map(str::to_owned),
        did_web_url: None,
        registered_trade_name: None,
        electronic_address: None,
    }
}

/// The envelope is additive-only and has no lens, so this is the property that
/// matters most: a passport stored before the field existed must still read.
#[test]
fn a_document_written_before_the_field_existed_reads_with_no_country() {
    let stored = serde_json::json!({
        "name": "GreenCell GmbH",
        "address": "Alexanderplatz 1, Berlin",
        "didWebUrl": null,
    });
    let info: ManufacturerInfo = serde_json::from_value(stored).expect("pre-field document reads");
    assert_eq!(info.country, None);
    assert_eq!(info.name, "GreenCell GmbH");
}

#[test]
fn the_wire_key_is_country() {
    let json = serde_json::to_value(manufacturer(Some("DE"))).unwrap();
    assert_eq!(json["country"], "DE");
}

#[test]
fn an_assigned_code_is_accepted() {
    assert!(manufacturer(Some("DE")).validate_country().is_ok());
    assert!(manufacturer(Some("BD")).validate_country().is_ok());
}

/// An unstated country is not an invalid one. Rejecting `None` would make the
/// field required, which the envelope's additive-only rule forbids.
#[test]
fn an_absent_country_is_not_an_error() {
    assert!(manufacturer(None).validate_country().is_ok());
}

/// The check is membership in the assigned ISO 3166-1 set, not two uppercase
/// letters. `XX` is the case that separates the two, and it is the one a
/// shape-only check lets through.
#[test]
fn a_well_formed_but_unassigned_code_is_refused() {
    assert_eq!(manufacturer(Some("XX")).validate_country(), Err("XX"));
    assert_eq!(manufacturer(Some("QZ")).validate_country(), Err("QZ"));
}

/// The failure mode this field exists to end: a country written into the
/// address, or an address written into the country.
#[test]
fn a_country_name_or_an_address_is_refused_where_a_code_belongs() {
    assert_eq!(
        manufacturer(Some("Germany")).validate_country(),
        Err("Germany")
    );
    assert_eq!(manufacturer(Some("de")).validate_country(), Err("de"));
    assert_eq!(
        manufacturer(Some("Alexanderplatz 1, Berlin")).validate_country(),
        Err("Alexanderplatz 1, Berlin")
    );
}

// ── ESPR Art. 27(6) contact details ──────────────────────────────────────────

/// The same additive property as the country test, for the two fields added
/// for Art. 27(6). A pre-field document is the common case, not an edge one:
/// every passport written before this release is one.
#[test]
fn a_document_written_before_the_contact_fields_existed_reads_with_neither() {
    let stored = serde_json::json!({
        "name": "GreenCell GmbH",
        "address": "Alexanderplatz 1, Berlin",
        "country": "DE",
    });
    let info: ManufacturerInfo = serde_json::from_value(stored).expect("pre-field document reads");
    assert_eq!(info.registered_trade_name, None);
    assert_eq!(info.electronic_address, None);
}

/// Art. 27(6) names the legal name and the trade name as alternatives precisely
/// because they differ, so the test uses two that do. A round-trip through one
/// `name` field could not tell them apart, which is the defect this closes.
#[test]
fn a_trade_name_distinct_from_the_legal_name_round_trips() {
    let mut info = manufacturer(Some("DE"));
    info.registered_trade_name = Some("GreenCell".to_owned());
    info.electronic_address = Some("contact@greencell.example".to_owned());

    let json = serde_json::to_value(&info).expect("serialises");
    assert_eq!(json["name"], "GreenCell GmbH");
    assert_eq!(json["registeredTradeName"], "GreenCell");
    assert_eq!(json["electronicAddress"], "contact@greencell.example");

    let back: ManufacturerInfo = serde_json::from_value(json).expect("deserialises");
    assert_eq!(back.registered_trade_name.as_deref(), Some("GreenCell"));
    assert_eq!(
        back.electronic_address.as_deref(),
        Some("contact@greencell.example")
    );
}

/// An absent field is omitted rather than serialised as `null`, matching every
/// other optional envelope field and keeping a stored document from gaining
/// two null keys it never had.
#[test]
fn unstated_contact_fields_are_omitted_not_nulled() {
    let json = serde_json::to_value(manufacturer(Some("DE"))).expect("serialises");
    assert!(
        json.get("registeredTradeName").is_none(),
        "expected the key to be absent, got {json}"
    );
    assert!(json.get("electronicAddress").is_none());
}

/// Art. 27(6)(a) puts the contact details on the **public part** of the
/// passport specifically, so they must survive redaction to the anonymous
/// audience. `manufacturer` carries no disclosure entry and therefore defaults
/// to `Public` — this pins that default rather than leaving it to be discovered
/// later by someone classifying the nested keys "for safety".
#[test]
fn the_public_view_keeps_the_contact_details() {
    use crate::access::redact_passport;
    use crate::disclosure::Audience;

    let mut passport = crate::test_support::fully_populated_passport();
    passport.manufacturer.registered_trade_name = Some("GreenCell".to_owned());
    passport.manufacturer.electronic_address = Some("contact@greencell.example".to_owned());

    let public = redact_passport(&passport, Audience::Public);
    let json = serde_json::to_value(&public).expect("serialises");

    assert_eq!(json["manufacturer"]["registeredTradeName"], "GreenCell");
    assert_eq!(
        json["manufacturer"]["electronicAddress"],
        "contact@greencell.example"
    );
}
