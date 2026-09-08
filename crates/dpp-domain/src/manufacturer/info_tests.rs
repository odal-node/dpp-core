//! What `ManufacturerInfo::country` accepts, and what a document written before
//! it existed does.

use super::ManufacturerInfo;

fn manufacturer(country: Option<&str>) -> ManufacturerInfo {
    ManufacturerInfo {
        name: "GreenCell GmbH".to_owned(),
        address: "Alexanderplatz 1, Berlin".to_owned(),
        country: country.map(str::to_owned),
        did_web_url: None,
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
