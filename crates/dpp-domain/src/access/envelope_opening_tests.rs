//! A product group opening envelope fields to the public: what it reaches, what
//! it cannot, and that it travels with the schema version that declared it.

use super::{DocumentScope, ProductGroupAccessPolicy, redact_passport};
use crate::disclosure::{Audience, Disclosure};
use crate::passport::Passport;
use crate::passport::tests::make_passport;
use crate::product_group::{ProductGroup, ProductGroupData};

fn battery_at(version: &str) -> Passport {
    let mut p = make_passport();
    p.product_group = ProductGroup::Battery;
    p.schema_version = version.into();
    p.batch_id = Some("LOT-A".into());
    p.serial_number = Some("SN-0001".into());
    p.product_group_data = Some(ProductGroupData::Battery(Box::new(
        crate::test_support::sample_battery_data(),
    )));
    p
}

/// Annex XIII point 1(a) of Regulation (EU) 2023/1542 makes the Art. 38(6)
/// identifying batch or serial number public, and the battery schema that says
/// so serves both to an anonymous reader.
#[test]
fn a_battery_serves_its_batch_and_serial_publicly() {
    let view = redact_passport(&battery_at("2.7.0"), Audience::Public).into_value();
    assert_eq!(view["batchId"], "LOT-A");
    assert_eq!(view["serialNumber"], "SN-0001");
}

/// The opening is the schema's, so a passport validated against an earlier
/// version keeps the classes its signatures were produced under.
#[test]
fn an_earlier_battery_version_keeps_them_restricted() {
    let view = redact_passport(&battery_at("2.6.0"), Audience::Public).into_value();
    assert!(view.get("batchId").is_none(), "{view}");
    assert!(view.get("serialNumber").is_none(), "{view}");
}

/// A product group whose schema opens nothing is untouched by another's.
#[test]
fn a_group_that_opens_nothing_keeps_the_default() {
    let mut p = make_passport();
    p.batch_id = Some("LOT-A".into());
    p.serial_number = Some("SN-0001".into());
    let view = redact_passport(&p, Audience::Public).into_value();
    assert!(view.get("batchId").is_none(), "{view}");
    assert!(view.get("serialNumber").is_none(), "{view}");
}

/// Opened at the envelope key only. The same name one level down, or inside
/// the product group's data, keeps whatever class it had.
#[test]
fn the_opening_is_anchored_to_the_top_level_envelope_key() {
    let policy = ProductGroupAccessPolicy::for_passport("battery", "2.7.0").expect("known");
    assert_eq!(
        policy.disclosure_for_path(&["serialNumber"], DocumentScope::Envelope),
        Disclosure::Public
    );
    assert_eq!(
        policy.disclosure_for_path(&["componentRefs", "serialNumber"], DocumentScope::Envelope),
        Disclosure::Restricted
    );
    assert_eq!(
        policy.disclosure_for_path(&["serialNumber"], DocumentScope::ProductGroupData),
        Disclosure::Restricted
    );
}

/// A schema may open only what the allow-list names. Anything else refuses the
/// whole policy, which fails closed exactly as an unreadable schema does.
#[test]
fn a_schema_opening_anything_else_is_refused() {
    let schema =
        |opened: &str| format!(r#"{{"properties": {{}}, "x-public-envelope-fields": [{opened}]}}"#);
    assert!(
        ProductGroupAccessPolicy::from_schema("battery", "9.9.9", &schema(r#""batchId""#))
            .is_some()
    );
    for forbidden in [r#""jwsSignature""#, r#""lifeStatus""#, r#""seal""#, "1"] {
        assert!(
            ProductGroupAccessPolicy::from_schema("battery", "9.9.9", &schema(forbidden)).is_none(),
            "{forbidden} must not be openable"
        );
    }
}
