//! The asset identity a shell carries, and what reaches each audience.

use super::super::*;
use super::common::*;
use crate::identity::AssetIdentity;
use dpp_domain::Audience;
use dpp_domain::identifier::ProductIdentifier;
use dpp_domain::{Gtin, ProductGroup};

/// 🚨 The defect this type removes, pinned per scheme.
///
/// Measured before it existed: `specificAssetIds = ["gtin=did:web:acme…"]` and
/// `globalAssetId = urn:odal-node:product:did:web:acme…` — a DID under a GS1
/// label, inside a URN whose namespace-specific string is another URI scheme.
#[test]
fn the_asset_id_is_named_after_the_scheme_that_issued_it() {
    let gs1 = ProductIdentifier::gs1(Gtin::parse("09506000134352").unwrap());
    let link = ProductIdentifier::identification_link("https://id.acme.example.com/b/1").unwrap();
    let did = ProductIdentifier::did("did:web:acme.example.com:b:1").unwrap();

    for (identifier, expected_name, expected_global) in [
        (&gs1, "gtin", "urn:odal-node:product:09506000134352"),
        (
            &link,
            "identificationLink",
            "https://id.acme.example.com/b/1",
        ),
        (&did, "did", "did:web:acme.example.com:b:1"),
    ] {
        let passport = minimal_passport(ProductGroup::Textile);
        let identity = AssetIdentity::Product(identifier);
        let (shell, _) =
            build_aas_from_passport(&passport, identity, Audience::Public).expect("masking");

        let named: Vec<(&str, &str)> = shell
            .asset_information
            .specific_asset_ids
            .iter()
            .map(|id| (id.name.as_str(), id.value.as_str()))
            .collect();
        assert!(
            named.contains(&(expected_name, identifier.as_str())),
            "{expected_name} must carry its own value: {named:?}"
        );
        assert!(
            !named
                .iter()
                .any(|(n, v)| *n == "gtin" && *v != identifier.as_str()),
            "nothing may be filed under `gtin` that is not one: {named:?}"
        );

        // 🚨 A value that is already a URI is used as-is. Wrapping one produces
        // `urn:odal-node:product:did:web:…`, and for a `urn:` value a URN
        // nested inside a URN.
        assert_eq!(
            shell.asset_information.global_asset_id, expected_global,
            "globalAssetId for {expected_name}"
        );
    }
}

#[test]
fn a_caller_chosen_key_may_not_impersonate_a_scheme() {
    // Without this the escape hatch reproduces the original defect exactly:
    // `Named { name: "gtin", value: <a DID> }` is the old behaviour spelled out.
    let passport = minimal_passport(ProductGroup::Textile);
    for reserved in RESERVED_ASSET_ID_NAMES {
        let identity = AssetIdentity::Named {
            name: reserved,
            value: "did:web:acme.example.com:b:1",
        };
        match build_aas_from_passport(&passport, identity, Audience::Public) {
            Err(AasError::ReservedAssetIdName(n)) => assert_eq!(n, reserved),
            other => panic!(
                "expected {reserved} to be refused, got {:?}",
                other.map(|_| ())
            ),
        }
    }
}

#[test]
fn a_caller_chosen_key_that_cannot_be_a_uri_is_refused() {
    // 🚨 Nothing downstream catches these. Measured against `aas-core3.0`
    // 1.1.4, the AAS reference implementation **accepts**
    // `urn:odal-node:product:asset 1`, `urn:odal-node:product:` and a value
    // with an embedded tab — it verifies `globalAssetId`'s length, not its
    // syntax. The name was already guarded and the value was not, which left
    // the guard asymmetric in the one type whose job is that the shell cannot
    // assert something false.
    let passport = minimal_passport(ProductGroup::Textile);
    for value in [
        "",
        "   ",
        "asset 1",
        "a	b",
        "line
break",
    ] {
        let identity = AssetIdentity::Named {
            name: "internalAssetKey",
            value,
        };
        match build_aas_from_passport(&passport, identity, Audience::Public) {
            Err(AasError::UnusableAssetIdentity(_)) => {}
            other => panic!(
                "expected {value:?} to be refused, got {:?}",
                other.map(|_| ())
            ),
        }
    }
}

/// A `Product` identity needs no such check, and this says why rather than
/// leaving it to be rediscovered: all three arms already refuse whitespace.
#[test]
fn a_scheme_issued_identity_cannot_carry_whitespace() {
    use dpp_domain::identifier::ProductIdentifier;

    assert!(ProductIdentifier::identification_link("https://acme.example.com/a b").is_err());
    assert!(ProductIdentifier::did("did:web:acme.example.com:a b").is_err());
    assert!(Gtin::parse("0950600013 52").is_err());
}

#[test]
fn a_caller_chosen_key_that_is_not_a_uri_is_wrapped() {
    // The safe direction: wrapping a non-URI yields a valid URN, while failing
    // to wrap one yields a `globalAssetId` that is not an identifier at all.
    let passport = minimal_passport(ProductGroup::Textile);
    let (shell, _) = build_aas_from_passport(
        &passport,
        AssetIdentity::Named {
            name: "internalAssetKey",
            value: "PLANT-7/LINE-2",
        },
        Audience::Public,
    )
    .expect("masking");
    assert_eq!(
        shell.asset_information.global_asset_id,
        "urn:odal-node:product:PLANT-7/LINE-2"
    );
}

#[test]
fn the_asset_kind_follows_the_registration_level() {
    // 🚨 `"Instance"` was hardcoded under a doc comment claiming a passport
    // "describes one manufactured item or batch, never a product type
    // definition". `Granularity::Model` is exactly that — one passport covering
    // every unit sharing a model's specifications.
    use dpp_domain::catalog::Granularity;
    for (granularity, expected) in [
        (Some(Granularity::Model), "Type"),
        (Some(Granularity::Batch), "Instance"),
        (Some(Granularity::Item), "Instance"),
        (None, "Instance"),
    ] {
        let mut passport = minimal_passport(ProductGroup::Textile);
        passport.granularity = granularity;
        let (shell, _) =
            build_aas_from_passport(&passport, identity_of(&passport), Audience::Public)
                .expect("masking");
        assert_eq!(
            shell.asset_information.asset_kind, expected,
            "granularity {granularity:?}"
        );
    }
}

#[test]
fn a_real_serial_number_travels_through_the_disclosure_seam() {
    // 🚨 `serial_number` is a non-public envelope field, so the masking strips
    // it before the builder ever sees it and a public shell reports no serial.
    // That is the correct answer, and it is also why the old code was wrong in
    // a second way: it emitted `serialId` unconditionally, carrying the
    // passport UUID — a value the disclosure policy never had a chance to
    // classify, because it was not the field it claimed to be.
    let mut passport = minimal_passport(ProductGroup::Textile);
    passport.serial_number = Some("SN-0001".into());

    let names = |audience| -> Vec<(String, String)> {
        let (shell, _) =
            build_aas_from_passport(&passport, identity_of(&passport), audience).expect("masking");
        shell
            .asset_information
            .specific_asset_ids
            .iter()
            .map(|id| (id.name.clone(), id.value.clone()))
            .collect()
    };

    let public = names(Audience::Public);
    assert!(
        !public.iter().any(|(n, _)| n == "serialId"),
        "an anonymous reader must not receive the unit serial: {public:?}"
    );
    assert!(
        public
            .iter()
            .any(|(n, v)| n == "passportId" && *v == passport.id.to_string()),
        "the passport id stays, under a name that says what it is: {public:?}"
    );

    let credentialed = names(Audience::LegitimateInterest);
    assert!(
        credentialed
            .iter()
            .any(|(n, v)| n == "serialId" && v == "SN-0001"),
        "a reader entitled to it gets the real serial, not a UUID: {credentialed:?}"
    );
}

/// The credentialed audience does get it, so the assertion above is a redaction
/// rather than the projection simply never emitting a batch id.
#[test]
fn batch_id_reaches_an_audience_entitled_to_it() {
    let passport = minimal_passport(ProductGroup::Textile);
    let (shell, _) = build_aas_from_passport(
        &passport,
        identity_of(&passport),
        Audience::LegitimateInterest,
    )
    .expect("masking");

    let names: Vec<&str> = shell
        .asset_information
        .specific_asset_ids
        .iter()
        .map(|id| id.name.as_str())
        .collect();
    assert!(
        names.contains(&"batchId"),
        "a legitimate-interest reader may see the batch identifier: {names:?}"
    );
}

#[test]
fn shell_id_contains_passport_id() {
    let passport = minimal_passport(ProductGroup::Battery);
    let id_str = passport.id.to_string();
    let (shell, _) = build_aas_from_passport(&passport, identity_of(&passport), Audience::Public)
        .expect("masking");
    assert!(shell.id.contains(&id_str));
}
