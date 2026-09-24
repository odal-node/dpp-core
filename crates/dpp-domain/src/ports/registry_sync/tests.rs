//! Behaviour of the registration request builder and the granularity mapping.

use super::*;
use crate::{
    passport::{ManufacturerInfo, Passport},
    status::PassportStatus,
};
use chrono::Utc;

fn make_published_passport() -> Passport {
    Passport {
        product_name: "Test".into(),
        manufacturer: ManufacturerInfo {
            name: "ACME".into(),
            address: "Berlin".into(),
            country: None,
            did_web_url: None,
            registered_trade_name: None,
            electronic_address: None,
        },
        status: PassportStatus::Published,
        qr_code_url: Some("https://id.odal-node.io/01/09506000134352".into()),
        jws_signature: Some("eyJ0eXAiOiJKV1QifQ.payload.sig".into()),
        published_at: Some(Utc::now()),
        placed_on_market_date: None,
        schema_version: "1.1.0".into(),
        retention_locked: true,
        operator_identifier: Some("did:web:acme.example.com".into()),
        product_group_data: Some(crate::product_group::ProductGroupData::Textile(Box::new(
            crate::test_support::sample_textile_data(),
        ))),
        facility: Some(crate::passport::FacilitySnapshot {
            scheme: "national".into(),
            value: "FAC-DE-001".into(),
            name: "Acme Plant".into(),
            country: "DE".into(),
            address: None,
        }),
        ..crate::test_support::sample_passport()
    }
}

/// The operator identity a test registration is filed under.
fn acme() -> RegisteringOperator<'static> {
    RegisteringOperator {
        legal_name: "Acme GmbH",
        country: "DE",
        identifier_scheme: "did",
    }
}

#[test]
fn from_published_passport_maps_all_fields() {
    let passport = make_published_passport();
    let req = RegistrationRequest::from_published_passport(
        &passport,
        acme(),
        RegistrationGranularity::Item,
    )
    .expect("the fixture passport is complete");

    assert_eq!(req.passport_id, passport.id);
    assert_eq!(req.operator_identifier, "did:web:acme.example.com");
    assert_eq!(req.facility_identifier, "FAC-DE-001");
    // The full facility descriptor is carried, not just the bare identifier.
    assert_eq!(
        req.facility.as_ref().map(|f| f.name.as_str()),
        Some("Acme Plant")
    );
    assert_eq!(
        req.facility.as_ref().map(|f| f.country.as_str()),
        Some("DE")
    );
    assert_eq!(req.product_category, "textile");
    assert_eq!(
        req.data_carrier_uri,
        "https://id.odal-node.io/01/09506000134352"
    );
    assert_eq!(req.schema_version, "1.1.0");
    assert!(req.jws_signature.is_some());
    assert!(req.published_at.is_some());
    assert_eq!(req.country_code, "DE");
    // The operator's legal name comes from operator config, never from the
    // passport's manufacturer block.
    assert_eq!(req.operator_name, "Acme GmbH");
    assert_ne!(
        req.operator_name, passport.manufacturer.name,
        "operator and manufacturer are distinct legal persons"
    );
    assert_eq!(req.granularity, RegistrationGranularity::Item);
}

/// 🚨 An incomplete passport is refused, naming **every** field it lacks.
///
/// This test used to be called `from_published_passport_empty_optionals_produce_empty_strings`
/// and asserted the opposite: that a passport with no operator identifier, no
/// facility and no carrier URL produced a request with `""` in all three. It
/// carried no reason, and the behaviour it pinned was the defect — a request
/// that looks complete, refused downstream by an error naming the scheme it was
/// given rather than the absence it was not.
///
/// All three at once, not the first: nothing has been sent yet, so the caller is
/// fixing their own passport and a round-trip per missing field is a worse
/// answer than a list.
#[test]
fn an_incomplete_passport_is_refused_with_every_missing_field_named() {
    let mut passport = make_published_passport();
    passport.operator_identifier = None;
    passport.facility = None;
    passport.qr_code_url = None;

    let refused = RegistrationRequest::from_published_passport(
        &passport,
        RegisteringOperator {
            legal_name: "",
            country: "",
            identifier_scheme: "",
        },
        RegistrationGranularity::Item,
    )
    .expect_err("a passport carrying none of the three cannot be registered");

    let fields: Vec<&str> = refused.errors.iter().map(|e| e.field.as_str()).collect();
    assert_eq!(
        fields,
        ["/operatorIdentifier", "/facility", "/qrCodeUrl"],
        "every missing field must be reported, not the first"
    );
}

/// And one missing field is one error — the list is not all-or-nothing in the
/// other direction either.
#[test]
fn a_passport_missing_only_its_carrier_url_reports_only_that() {
    let mut passport = make_published_passport();
    passport.qr_code_url = None;

    let refused = RegistrationRequest::from_published_passport(
        &passport,
        acme(),
        RegistrationGranularity::Item,
    )
    .expect_err("the carrier URI is what a registration resolves to");

    assert_eq!(refused.errors.len(), 1);
    assert_eq!(refused.errors[0].field, "/qrCodeUrl");
}

#[test]
fn registry_status_serde_round_trip() {
    let statuses = vec![
        RegistryStatus::Pending,
        RegistryStatus::Registered,
        RegistryStatus::Rejected,
        RegistryStatus::SuspendedByAuthority,
        RegistryStatus::Deactivated,
    ];
    for status in statuses {
        let json = serde_json::to_string(&status).unwrap();
        let back: RegistryStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(status, back);
    }
}

/// Art. 8(4): an item-level registration links the model identifier where a
/// model design exists. It exists for batteries, and the registration must
/// carry it rather than claiming the product has none.
#[test]
fn the_model_identifier_reaches_the_registration() {
    use crate::product_group::ProductGroupData;

    let mut passport = make_published_passport();
    passport.product_group_data = Some(ProductGroupData::Battery(Box::new(
        crate::product_group::BatteryData {
            battery_model_id: Some("BM-4815".into()),
            ..crate::test_support::sample_battery_data()
        },
    )));

    let req = RegistrationRequest::from_published_passport(
        &passport,
        acme(),
        RegistrationGranularity::Item,
    )
    .expect("the fixture passport is complete");
    assert_eq!(req.model_id.as_deref(), Some("BM-4815"));
}

/// Absent is a substantive answer — the lawful "no model design exists" —
/// and must not be confused with a lookup that was never wired.
#[test]
fn a_product_group_without_a_model_identifier_reports_none() {
    // Textile: it identifies a product, and carries no model design. The case
    // used to be built by clearing `product_group_data` entirely, which now
    // refuses — a passport with no product group identifies nothing, which is a
    // different statement from "this product has no model".
    let passport = make_published_passport();
    assert!(
        RegistrationRequest::from_published_passport(
            &passport,
            acme(),
            RegistrationGranularity::Item,
        )
        .expect("the fixture passport is complete")
        .model_id
        .is_none()
    );
}

/// 🚨 The identifier reaches the port, whichever clause 5 scheme issued it.
///
/// Before this field the request carried nothing identifying the product, so an
/// adapter scraped a GTIN out of the carrier URI and fell back to the internal
/// passport UUID when there was none — which is every scheme 2 and 3 passport.
/// That registers a product with a public authority under a value meaningless
/// outside this node, and no structural check catches it.
#[test]
fn every_clause_5_scheme_reaches_the_request() {
    use crate::identifier::ProductIdentifier;
    use crate::product_group::ProductGroupData;

    for identifier in [
        ProductIdentifier::gs1(crate::identifier::Gtin::parse("09506000134352").unwrap()),
        ProductIdentifier::identification_link("https://id.acme.example.com/p/1").unwrap(),
        ProductIdentifier::did("did:web:acme.example.com:p:1").unwrap(),
    ] {
        let mut passport = make_published_passport();
        let mut textile = crate::test_support::sample_textile_data();
        textile.product_identifier = identifier.clone();
        passport.product_group_data = Some(ProductGroupData::Textile(Box::new(textile)));

        let req = RegistrationRequest::from_published_passport(
            &passport,
            acme(),
            RegistrationGranularity::Item,
        )
        .expect("a passport that identifies itself can be registered");

        assert_eq!(
            req.product_identifier.as_ref(),
            Some(&identifier),
            "the identifier must travel, not be re-derived from the carrier"
        );
    }
}

/// A product group that identifies no single product cannot be registered.
///
/// `UnsoldGoods` is the known case and is not a defect: an Art. 24–25 discard
/// disclosure covers a financial year across many products. Refused here rather
/// than allowed through with nothing in the field, so the case is named instead
/// of arriving at an adapter that has to invent something.
#[test]
fn a_product_group_that_identifies_nothing_is_refused() {
    use crate::product_group::ProductGroupData;

    let mut passport = make_published_passport();
    passport.product_group_data = Some(ProductGroupData::UnsoldGoods(
        crate::test_support::sample_unsold_goods_report(),
    ));

    let refused = RegistrationRequest::from_published_passport(
        &passport,
        acme(),
        RegistrationGranularity::Item,
    )
    .expect_err("a discard disclosure is not a product registration");

    assert_eq!(refused.errors.len(), 1);
    assert_eq!(
        refused.errors[0].field,
        "/productGroupData/productIdentifier"
    );
}

/// A passport for a product group this build has no typed variant for, with
/// `product_identifier` as its payload's `productIdentifier`.
fn untyped_passport(product_identifier: serde_json::Value) -> Passport {
    use crate::product_group::{ProductGroup, ProductGroupData};

    let mut passport = make_published_passport();
    passport.product_group = ProductGroup::Other("photovoltaic".into());
    passport.product_group_data = ProductGroupData::other(serde_json::json!({
        "productGroup": "photovoltaic",
        "productIdentifier": product_identifier,
    }));
    passport
}

/// The defect: an untyped product group carrying a valid identifier on the
/// wire was refused, because nothing read one out of untyped data — so a group
/// added to the catalog after this crate shipped needed a release to register.
#[test]
fn an_untyped_product_group_registers_on_the_identifier_it_carries() {
    let passport = untyped_passport(serde_json::json!({
        "scheme": "gs1",
        "gtin": "09506000134352",
    }));

    let req = RegistrationRequest::from_published_passport(
        &passport,
        acme(),
        RegistrationGranularity::Item,
    )
    .expect("a valid identifier registers whether or not the group is typed");

    let identifier = req.product_identifier.expect("carried");
    assert_eq!(identifier.as_str(), "09506000134352");
}

/// Reading the untyped payload does not make it lenient: an identifier that
/// fails EN 18219 clause 5 is refused exactly as an absent one is.
#[test]
fn an_untyped_product_group_with_a_malformed_identifier_is_refused() {
    let passport = untyped_passport(serde_json::json!({
        "scheme": "gs1",
        "gtin": "09506000134351",
    }));

    let refused = RegistrationRequest::from_published_passport(
        &passport,
        acme(),
        RegistrationGranularity::Item,
    )
    .expect_err("a bad check digit is no identifier");
    assert_eq!(
        refused.errors[0].field,
        "/productGroupData/productIdentifier"
    );
}

/// 🚨 `Some("")` is the same absence wearing an `Option::Some`.
///
/// `Passport`'s fields are public and it deserialises from stored documents, so
/// nothing stops a blank value being written where `None` belongs. Checking
/// presence alone would have left this constructor doing exactly what it was
/// changed to stop doing — producing a request that looks complete and carries
/// nothing — one layer in from where the defect was found.
///
/// Whitespace counts as blank: a value of `" "` identifies no more than `""`
/// does, and is the form a trimmed-input bug actually produces.
#[test]
fn a_present_but_blank_field_is_as_absent_as_a_missing_one() {
    let mut passport = make_published_passport();
    passport.operator_identifier = Some(String::new());
    passport.qr_code_url = Some("   ".into());
    if let Some(facility) = passport.facility.as_mut() {
        facility.value = String::new();
    }

    let refused = RegistrationRequest::from_published_passport(
        &passport,
        acme(),
        RegistrationGranularity::Item,
    )
    .expect_err("blank is not a value the passport carried");

    let fields: Vec<&str> = refused.errors.iter().map(|e| e.field.as_str()).collect();
    assert_eq!(fields, ["/operatorIdentifier", "/facility", "/qrCodeUrl"]);
}
