//! How a product identity is formed and compared.

use super::identity::*;
use crate::passport::ManufacturerInfo;
use crate::passport::Passport;
use crate::product_group::ProductGroup;
use crate::product_group::{ProductGroupData, TextileData};

fn base_passport(
    product_group: ProductGroup,
    product_group_data: Option<ProductGroupData>,
) -> Passport {
    Passport {
        batch_id: Some("BATCH-1".into()),
        serial_number: None,
        product_name: "Test".into(),
        product_group,
        manufacturer: ManufacturerInfo {
            name: "Acme".into(),
            address: "1 Street".into(),
            country: None,
            did_web_url: None,
            registered_trade_name: None,
            electronic_address: None,
        },
        product_group_data,
        ..crate::test_support::sample_passport()
    }
}

fn battery_data() -> ProductGroupData {
    ProductGroupData::Battery(Box::new(crate::test_support::sample_battery_data()))
}

#[test]
fn battery_passport_yields_identity() {
    let p = base_passport(ProductGroup::Battery, Some(battery_data()));
    let id = ProductIdentity::from_passport(&p).expect("battery has a gtin");
    assert_eq!(id.product_group, ProductGroup::Battery);
    assert_eq!(id.identifier, "09506000134352");
    assert_eq!(id.batch_id.as_deref(), Some("BATCH-1"));
}

#[test]
fn textile_passport_yields_identity() {
    let textile_data = ProductGroupData::Textile(Box::new(TextileData {
        country_of_origin: "BD".into(),
        care_instructions: "wash".into(),
        chemical_compliance_standard: "OEKO-TEX 100".into(),
        ..crate::test_support::sample_textile_data()
    }));
    let p = base_passport(ProductGroup::Textile, Some(textile_data));
    let id = ProductIdentity::from_passport(&p).expect("textile has a gtin");
    assert_eq!(id.product_group, ProductGroup::Textile);
    assert_eq!(id.identifier, "09506000134352");
}

#[test]
fn no_product_group_data_yields_no_identity() {
    let p = base_passport(ProductGroup::Battery, None);
    assert!(ProductIdentity::from_passport(&p).is_none());
}

/// 🚨 A passport identified under EN 18219 scheme 2 or 3 must still produce a
/// matching key.
///
/// `from_passport` read `ProductGroupData::gtin()`, which answers `None` for
/// both self-issuing schemes — so a perfectly well-identified passport yielded
/// no identity, the import delta-matcher found nothing to match against, and the
/// import created a **duplicate** instead of updating the record it was looking
/// at. Silent, and it corrupts the one thing the key exists to prevent.
#[test]
fn a_self_issued_identifier_still_yields_a_matching_identity() {
    use crate::identifier::ProductIdentifier;

    let cases = [
        (
            ProductIdentifier::did("did:web:example.com:p:1").unwrap(),
            "did:web:example.com:p:1",
        ),
        (
            ProductIdentifier::identification_link("https://id.example.com/p/1").unwrap(),
            "https://id.example.com/p/1",
        ),
    ];

    for (identifier, expected) in cases {
        let mut textile = crate::test_support::sample_textile_data();
        textile.product_identifier = identifier;
        let p = base_passport(
            ProductGroup::Textile,
            Some(ProductGroupData::Textile(Box::new(textile))),
        );

        let id = ProductIdentity::from_passport(&p)
            .expect("a scheme 2 or 3 passport is identified and must yield a key");
        assert_eq!(id.identifier, expected);
        assert_eq!(id.product_group, ProductGroup::Textile);
    }
}

/// The other half: a group that genuinely identifies no single product still
/// yields nothing, so the fix above did not turn `None` into "always Some".
#[test]
fn a_group_with_no_identifier_still_yields_no_identity() {
    let p = base_passport(
        ProductGroup::Other("hypothetical".to_owned()),
        Some(ProductGroupData::Other {
            product_group: "hypothetical".to_owned(),
            data: serde_json::json!({ "productGroup": "hypothetical" }),
        }),
    );
    assert!(ProductIdentity::from_passport(&p).is_none());
}

/// 🚨 Two item-level passports must not share an identity.
///
/// Before `serial_number` was part of the key they did: two published units of
/// the same batch produced the *identical* `ProductIdentity`. `Granularity`
/// admits `Item`, so that is a state the model invites rather than an exotic
/// one — and the import delta-matcher keys on this to decide create vs update
/// **before any write**, so a second unit was classified as a change to the
/// first.
#[test]
fn two_units_of_one_batch_are_two_identities() {
    let unit = |serial: &str| {
        let mut p = crate::test_support::sample_passport();
        p.product_group = crate::ProductGroup::Battery;
        p.product_group_data = Some(crate::ProductGroupData::Battery(Box::new(
            crate::test_support::sample_battery_data(),
        )));
        p.batch_id = Some("LOT-A".into());
        p.serial_number = Some(serial.to_owned());
        p
    };

    let a = ProductIdentity::from_passport(&unit("SN-0001")).expect("an identity");
    let b = ProductIdentity::from_passport(&unit("SN-0002")).expect("an identity");

    assert_eq!(a.identifier, b.identifier, "same product");
    assert_eq!(a.batch_id, b.batch_id, "same production run");
    assert_eq!(a.serial_number.as_deref(), Some("SN-0001"));
    assert_ne!(a, b, "different units are different identities");
}

/// And the level above still collapses correctly: a batch record and a unit of
/// that batch are not the same identity either.
#[test]
fn a_batch_record_and_a_unit_of_it_are_two_identities() {
    let mut batch = crate::test_support::sample_passport();
    batch.product_group = crate::ProductGroup::Battery;
    batch.product_group_data = Some(crate::ProductGroupData::Battery(Box::new(
        crate::test_support::sample_battery_data(),
    )));
    batch.batch_id = Some("LOT-A".into());

    let mut unit = batch.clone();
    unit.serial_number = Some("SN-0001".into());

    let b = ProductIdentity::from_passport(&batch).expect("an identity");
    let u = ProductIdentity::from_passport(&unit).expect("an identity");
    assert_eq!(b.serial_number, None);
    assert_ne!(b, u);
}
