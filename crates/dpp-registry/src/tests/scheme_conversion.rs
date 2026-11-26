//! Conversion of each clause 5 scheme to its registry scheme.

use super::super::*;
use super::common::*;

/// Each clause 5 scheme converts to its own scheme value, and carries the
/// identifier verbatim.
///
/// 🚨 The GS1 arm yields the **bare 14-digit GTIN**, not a Digital Link URL.
/// Building the URL needs a resolver host, which this tier does not know — and
/// a conversion that guessed one would put a resolver nobody configured into a
/// registration.
#[test]
fn each_clause_5_scheme_converts_to_its_own_registry_scheme() {
    use dpp_domain::identifier::ProductIdentifier as Clause5;

    let cases = [
        (
            Clause5::gs1(dpp_domain::Gtin::parse("09506000134352").unwrap()),
            SCHEME_GTIN,
            "09506000134352",
        ),
        (
            Clause5::identification_link("https://id.example.com/p/1").unwrap(),
            SCHEME_IDENTIFICATION_LINK,
            "https://id.example.com/p/1",
        ),
        (
            Clause5::did("did:web:example.com:p:1").unwrap(),
            SCHEME_DID,
            "did:web:example.com:p:1",
        ),
    ];

    for (clause5, expected_scheme, expected_value) in cases {
        let converted = ProductIdentifier::try_from(&clause5).expect("a mapped scheme");
        assert_eq!(converted.scheme, expected_scheme);
        assert_eq!(converted.value, expected_value);
        assert_eq!(converted.label, None);
    }
}

/// The scheme values must be distinct, or two schemes register as one.
///
/// Worth its own assertion because the failure is invisible: a copy-paste
/// leaving two constants equal produces conversions that validate, serialise and
/// round-trip, while filing a DID and a link under one name.
#[test]
fn no_two_schemes_share_a_registry_value() {
    let mut values: Vec<&str> = PRODUCT_SCHEME_BASIS.iter().map(|(v, _)| *v).collect();
    let count = values.len();
    values.sort_unstable();
    values.dedup();
    assert_eq!(
        values.len(),
        count,
        "two clause 5 schemes share a scheme value"
    );
}

/// 🚨 None of the three is observed, and the table says so where code can read it.
///
/// `"gtin"` and `"did"` have in-crate precedent, which is a convention of ours
/// and not evidence about the registry; scheme 2's has none at all. If one is
/// ever read off a real response this fails, which is the point — the upgrade
/// should be deliberate.
#[test]
fn every_scheme_value_is_assumed_not_observed() {
    for (value, basis) in PRODUCT_SCHEME_BASIS {
        assert!(
            !basis.is_observed(),
            "{value} claims to be observed; no registry response has been seen naming any scheme"
        );
    }
}

/// A converted scheme 1 identifier still passes the mod-10 check the registry
/// crate applies to `"gtin"` — the one scheme value with behaviour attached.
#[test]
fn a_converted_gtin_still_validates_as_one() {
    use dpp_domain::identifier::ProductIdentifier as Clause5;

    let good = Clause5::gs1(dpp_domain::Gtin::parse("09506000134352").unwrap());
    let converted = ProductIdentifier::try_from(&good).unwrap();
    assert!(converted.validate().is_ok());

    // And the check really is keyed on the scheme string: relabel a DID as a
    // GTIN and the same validator refuses it, which is why the conversion must
    // not be left to each consumer to spell.
    let did = Clause5::did("did:web:example.com:p:1").unwrap();
    let mut mislabelled = ProductIdentifier::try_from(&did).unwrap();
    mislabelled.scheme = SCHEME_GTIN.to_owned();
    assert!(matches!(
        mislabelled.validate(),
        Err(RegistryValidationError::InvalidGtin { .. })
    ));
}

// ── A blank value is no value, under every scheme ────────────────────────────

/// 🚨 The defect all four identifier types shared: *not verifying a scheme's
/// structure* also meant not noticing there was no value.
///
/// `validate_operator_scheme` returned `true` for `"did"` and for every
/// unrecognised scheme regardless of the value, so a blank identifier passed and
/// would have been submitted as though it identified someone. An empty operator
/// identifier failed under `"vat"` only because `has_country_prefix` happens to
/// reject one — whether an absent value was caught at all depended on which
/// scheme the operator was configured with.
///
/// Both forms, per type. `" "` is what a trimmed-input bug produces and is the
/// one `is_empty` alone cannot see.
#[test]
fn a_blank_value_is_refused_under_every_scheme() {
    for blank in ["", "   "] {
        // The scheme with no structural check at all, and the one that made the
        // original defect invisible.
        for scheme in ["did", "something-nobody-taught-us"] {
            let operator = OperatorIdentifier {
                scheme: scheme.into(),
                value: blank.into(),
                name: "Example GmbH".into(),
                country: "DE".into(),
                did: None,
            };
            assert!(
                matches!(
                    operator.validate(),
                    Err(RegistryValidationError::InvalidOperatorId { .. })
                ),
                "an operator identifier of {blank:?} under {scheme:?} was accepted"
            );
        }

        let product = ProductIdentifier {
            scheme: "did".into(),
            value: blank.into(),
            label: None,
        };
        assert!(
            matches!(
                product.validate(),
                Err(RegistryValidationError::MissingRequiredField(ref f)) if f == "productId.value"
            ),
            "a product identifier of {blank:?} was accepted"
        );

        let item = ProductItemIdentifier {
            scheme: "serial".into(),
            value: blank.into(),
            batch_id: None,
        };
        assert!(
            matches!(
                item.validate(),
                Err(RegistryValidationError::MissingRequiredField(ref f)) if f == "itemId.value"
            ),
            "an item identifier of {blank:?} was accepted"
        );

        let facility = FacilityIdentifier {
            scheme: "national".into(),
            value: blank.into(),
            name: Some("Plant".into()),
            country: "DE".into(),
            address: None,
        };
        assert!(
            matches!(
                facility.validate(),
                Err(RegistryValidationError::MissingRequiredField(ref f)) if f == "facilityId.value"
            ),
            "a facility identifier of {blank:?} was accepted"
        );
    }
}

/// And a stated value under an unchecked scheme still passes — the fix requires
/// a value, it does not start verifying schemes nobody has taught the validator
/// about.
#[test]
fn an_unchecked_scheme_still_accepts_a_stated_value() {
    let operator = OperatorIdentifier {
        scheme: "something-nobody-taught-us".into(),
        value: "OP-4471".into(),
        name: "Example GmbH".into(),
        country: "DE".into(),
        did: None,
    };
    assert!(operator.validate().is_ok());

    let product = ProductIdentifier {
        scheme: "did".into(),
        value: "did:web:example.com:p:1".into(),
        label: None,
    };
    assert!(product.validate().is_ok());
}

/// 🚨 A registration carries operator and facility identifiers that follow an EN
/// 18219 clause 6 scheme.
///
/// An `eori`, a `vat` and a `did` identifier are well formed, and `validate` has
/// always accepted them. None has a clause 6 route this build can establish, so a
/// registration is where they stop: it is a separate error from a malformed value,
/// because the value is fine.
#[test]
fn a_registration_whose_operator_identifier_has_no_clause_6_route_is_refused() {
    for (scheme, value) in [
        ("eori", "DE123456789012345"),
        ("vat", "DE123456789"),
        ("did", "did:web:example.com"),
    ] {
        let mut payload = sample_payload();
        payload.operator_id.scheme = scheme.into();
        payload.operator_id.value = value.into();

        assert!(
            payload.operator_id.validate().is_ok(),
            "{scheme} is well formed and must still validate as an identifier"
        );
        assert_eq!(
            payload.validate(),
            Err(RegistryValidationError::NoClause6Route {
                subject: "operator",
                scheme: scheme.into(),
            }),
            "{scheme} was registered"
        );
    }
}

#[test]
fn a_registration_whose_facility_identifier_has_no_clause_6_route_is_refused() {
    let mut payload = sample_payload();
    payload.facility_id.scheme = "national".into();
    payload.facility_id.value = "DE-BY-0042".into();

    assert!(payload.facility_id.validate().is_ok());
    assert_eq!(
        payload.validate(),
        Err(RegistryValidationError::NoClause6Route {
            subject: "facility",
            scheme: "national".into(),
        })
    );
}

/// What has a route is accepted, as an operator and as a facility.
#[test]
fn an_lei_a_duns_number_and_a_gln_are_accepted_in_a_registration() {
    for (scheme, value) in [("lei", "529900T8BM49AURSDO55"), ("duns", "123456789")] {
        let mut payload = sample_payload();
        payload.operator_id.scheme = scheme.into();
        payload.operator_id.value = value.into();
        assert!(payload.validate().is_ok(), "{scheme} was refused");
    }

    let mut payload = sample_payload();
    payload.operator_id.scheme = "gln".into();
    payload.operator_id.value = "4012345000009".into();
    assert!(payload.validate().is_ok(), "a party GLN was refused");

    let mut payload = sample_payload();
    payload.facility_id.scheme = "lei".into();
    payload.facility_id.value = "529900T8BM49AURSDO55".into();
    assert!(payload.validate().is_ok(), "an LEI facility was refused");
}

/// A malformed value is reported as malformed, not as lacking a route: the route
/// check comes after the shape check.
#[test]
fn a_malformed_value_is_not_reported_as_a_missing_route() {
    let mut payload = sample_payload();
    payload.operator_id.value = "529900T8BM49AURSDO56".into();
    assert!(matches!(
        payload.validate(),
        Err(RegistryValidationError::InvalidOperatorId { .. })
    ));
}

/// A facility's malformed LEI or D-U-N-S number is a malformed facility
/// identifier, under its own scheme, and not a malformed GLN.
#[test]
fn a_malformed_facility_lei_or_duns_number_is_not_reported_as_a_gln() {
    for (scheme, value) in [("lei", "529900T8BM49AURSDO56"), ("duns", "12345678")] {
        let mut payload = sample_payload();
        payload.facility_id.scheme = scheme.into();
        payload.facility_id.value = value.into();
        assert_eq!(
            payload.validate(),
            Err(RegistryValidationError::InvalidFacilityId {
                scheme: scheme.into(),
                value: value.into(),
            }),
            "{scheme}"
        );
    }
}
