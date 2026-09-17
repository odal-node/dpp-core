//! End-to-end: a published passport becomes a registry submission.
//!
//! # Why this file exists
//!
//! `dpp-registry` has eighty-odd tests and, until this file, **`dpp-tests` did
//! not depend on it at all**. Every registry test was inside the crate that
//! defines the types, asserting them against each other. That is the shape of
//! suite that proves a crate is self-consistent and proves nothing about whether
//! it composes — the same distinction `RegistryBasis` draws for evidence, one
//! level up.
//!
//! What landed across this release touches both sides of that seam:
//!
//! - a passport can be identified without GS1, under any EN 18219 clause 5
//!   scheme;
//! - a registration carries the Annex III(l) service provider beside the
//!   back-up URL, and refuses a URL with no party named;
//! - a submission is the unit that travels and fails, up to a hundred passports
//!   at a time;
//! - the Art. 9 proof of registration reports back.
//!
//! Each was reviewed and tested in isolation. This asserts they meet.

use chrono::Utc;
use dpp_domain::identifier::ProductIdentifier as SchemeIdentifier;
use dpp_domain::ports::registry_sync::{
    RegisteringOperator, RegistrationGranularity, RegistrationRequest, ServiceProviderRef,
};
use dpp_domain::{FibreEntry, Gtin, PassportStatus, ProductGroup, ProductGroupData, TextileData};
use dpp_registry::{
    FacilityIdentifier, Granularity, OperatorIdentifier, ProductIdentifier, ProductItemIdentifier,
    RegistrationLevel, RegistrationPayload, RegistrationSubmission, RegistryValidationError,
    ServiceProviderReference,
};
use dpp_tests::fixtures;

/// A published textile passport carrying `identifier`.
///
/// Only the mandatory v1.0.0 fields: what travels to the registry is the
/// envelope and the identifier, not the product group payload's optional depth.
fn published_passport(identifier: SchemeIdentifier) -> dpp_domain::Passport {
    let textile = TextileData {
        product_identifier: identifier,
        fibre_composition: vec![FibreEntry {
            fibre: "cotton".into(),
            pct: 100.0,
            country_of_origin: Some("IN".into()),
        }],
        country_of_origin: "BD".into(),
        care_instructions: "Machine wash 30°C".into(),
        chemical_compliance_standard: "OEKO-TEX 100".into(),
        // Every optional field spelled out rather than defaulted. `TextileData`
        // has no `Default` and should not gain one for a test's convenience:
        // `ProductIdentifier` has none either, on purpose, because defaulting an
        // identifier means choosing a scheme on the operator's behalf.
        recycled_content_pct: None,
        carbon_footprint_kg_co2e: None,
        water_use_litres: None,
        microplastic_shedding_mg_per_wash: None,
        repair_score: None,
        durability_score: None,
        expected_wash_cycles: None,
        country_of_raw_material_origin: None,
        svhc_substances: None,
        allergens: None,
        substances_of_concern: None,
        recyclability_class: None,
        end_of_life_instructions: None,
        reuse_condition: None,
        prior_use_cycles: None,
        disassembly_instructions: None,
        spare_parts_available: None,
        product_weight_grams: None,
        repair_history_url: None,
        repair_count: None,
        pef_score: None,
    };
    let mut passport = fixtures::base_passport(
        ProductGroup::Textile,
        ProductGroupData::Textile(Box::new(textile)),
        "1.3.0",
    );
    passport.status = PassportStatus::Published;
    passport.published_at = Some(Utc::now());
    // The three a registration cannot be built without. `base_passport` leaves
    // them unset, and `from_published_passport` now refuses rather than
    // defaulting them to `""` — see
    // `a_passport_missing_its_operator_identifier_cannot_be_registered`.
    passport.operator_identifier = Some("DE123456789".into());
    passport.qr_code_url = Some("https://id.ecotextile.de/01/09506000134352/21/ABC123".into());
    passport.facility = Some(dpp_domain::passport::FacilitySnapshot {
        scheme: "gln".into(),
        value: "4012345000009".into(),
        name: "Dhaka Unit 3".into(),
        country: "BD".into(),
        address: None,
    });
    passport
}

fn operator() -> RegisteringOperator<'static> {
    RegisteringOperator {
        legal_name: "EcoTextile GmbH",
        country: "DE",
        identifier_scheme: "vat",
    }
}

/// Core's conversion, which this test used to have to write for itself.
///
/// The hand-written version lived here with a note saying no helper existed —
/// and writing it was the finding: the `scheme` string is where an invented
/// mapping goes wrong *without failing*, because `ProductIdentifier::validate`
/// checks structure only when the scheme is `"gtin"`. Now that `TryFrom` exists,
/// this calls it, which is also the check that the conversion is the one a real
/// consumer needs rather than one shaped to its own tests.
fn registry_identifier(identifier: &SchemeIdentifier) -> ProductIdentifier {
    ProductIdentifier::try_from(identifier).expect("every clause 5 scheme this test uses is mapped")
}

/// 🚨 The identifier a real adapter has: the one **on the request**.
///
/// The payload builder below used to take the identifier as a separate argument,
/// handed to it beside the request — which quietly assumed a consumer could get
/// it from somewhere. It could not: `RegistrationRequest` carried no product
/// identifier, so the only adapter doing this scraped a GTIN out of the carrier
/// URI and fell back to the internal passport UUID when there was none, which is
/// every scheme 2 and 3 passport. Taking it from the request is what makes this
/// test exercise the path a consumer actually has.
fn identifier_on(request: &RegistrationRequest) -> ProductIdentifier {
    registry_identifier(
        request
            .product_identifier
            .as_ref()
            .expect("the constructor refuses a passport that identifies nothing"),
    )
}

fn payload_from(request: &RegistrationRequest) -> RegistrationPayload {
    RegistrationPayload {
        passport_id: request.passport_id.0,
        product_id: identifier_on(request),
        level: RegistrationLevel::new(Granularity::Item).with_model("MODEL-1"),
        item_id: Some(ProductItemIdentifier {
            scheme: "serial".into(),
            value: "ABC123".into(),
            batch_id: None,
        }),
        facility_id: FacilityIdentifier {
            scheme: "gln".into(),
            value: "4012345000009".into(),
            name: Some("Dhaka Unit 3".into()),
            country: "BD".into(),
            address: None,
        },
        operator_id: OperatorIdentifier {
            scheme: request.operator_identifier_scheme.clone(),
            value: request.operator_identifier.clone(),
            name: request.operator_name.clone(),
            country: request.country_code.clone(),
            did: None,
        },
        product_group: request.product_category.clone(),
        schema_version: request.schema_version.clone(),
        digital_link_url: request.data_carrier_uri.clone(),
        published_at: request.published_at.unwrap_or_else(Utc::now),
        jws_signature: request.jws_signature.clone(),
        commodity_code: request.commodity_code.clone(),
        backup_url: request.backup_url.clone(),
        service_provider: request
            .service_provider
            .as_ref()
            .map(|p| ServiceProviderReference {
                name: p.name.clone(),
                scheme: p.scheme.clone(),
                value: p.value.clone(),
                country: p.country.clone(),
            }),
    }
}

/// 🚨 The composition this release is *for*: a passport with no GTIN reaches the
/// registry.
///
/// Before the identifier work, `ProductGroupData` required a `Gtin`, so a
/// passport without GS1 membership could not exist to be registered. All three
/// clause 5 schemes must now travel the whole way — and each must arrive naming
/// **its own** scheme, because the registry stores what it is told and a DID
/// filed as a GTIN is a false statement no structural check catches.
#[test]
fn a_passport_under_any_en_18219_scheme_reaches_a_valid_submission() {
    let cases = [
        (
            SchemeIdentifier::gs1(Gtin::parse("09506000134352").unwrap()),
            dpp_registry::SCHEME_GTIN,
            "09506000134352",
        ),
        (
            SchemeIdentifier::identification_link("https://id.ecotextile.de/p/1").unwrap(),
            dpp_registry::SCHEME_IDENTIFICATION_LINK,
            "https://id.ecotextile.de/p/1",
        ),
        (
            SchemeIdentifier::did("did:web:ecotextile.de:p:1").unwrap(),
            dpp_registry::SCHEME_DID,
            "did:web:ecotextile.de:p:1",
        ),
    ];

    for (identifier, expected_scheme, expected_value) in cases {
        let passport = published_passport(identifier.clone());
        let request = RegistrationRequest::from_published_passport(
            &passport,
            operator(),
            RegistrationGranularity::Item,
        )
        .expect("the fixture passport carries all three");
        let payload = payload_from(&request);

        assert_eq!(payload.product_id.scheme, expected_scheme);
        assert_eq!(payload.product_id.value, expected_value);

        let submission = RegistrationSubmission::single(payload);
        assert!(
            submission.validate().is_ok(),
            "{expected_scheme} did not survive the trip: {:?}",
            submission.validate()
        );
    }
}

/// The Annex III(l) pairing, across the seam.
///
/// The port request carries `ServiceProviderRef` and the registry payload
/// carries `ServiceProviderReference` — two shapes for one party, on either side
/// of a crate boundary. A back-up URL that loses its provider in the mapping
/// produces a payload the registry refuses, and this is where that would show.
#[test]
fn a_back_up_declared_on_the_port_request_carries_its_provider_through() {
    let identifier = SchemeIdentifier::did("did:web:ecotextile.de:p:1").unwrap();
    let passport = published_passport(identifier.clone());
    let mut request = RegistrationRequest::from_published_passport(
        &passport,
        operator(),
        RegistrationGranularity::Item,
    )
    .expect("the fixture passport carries all three");
    request.backup_url = Some("https://backup.example.com/dpp/1.json".into());
    request.service_provider = Some(ServiceProviderRef::named("Example Backup GmbH"));

    let payload = payload_from(&request);
    assert!(payload.validate().is_ok(), "{:?}", payload.validate());

    // …and dropping the provider on the way across is refused, rather than
    // silently registering a back-up hosted by nobody.
    let mut orphaned = payload.clone();
    orphaned.service_provider = None;
    assert!(matches!(
        orphaned.validate(),
        Err(RegistryValidationError::MissingRequiredField(ref f)) if f == "serviceProvider"
    ));
}

/// 🚨 All-or-nothing, composed: ninety-nine good passports and one bad one.
///
/// Each passport is individually valid by construction except the last, which
/// loses its item identifier — an Art. 8(1) requirement at item granularity.
/// The registry rejects the submission entire, and a consumer that tested one
/// passport at a time would never see it.
#[test]
fn one_bad_passport_refuses_a_hundred_registrations() {
    let identifier = SchemeIdentifier::did("did:web:ecotextile.de:p:1").unwrap();
    let passport = published_passport(identifier.clone());
    let request = RegistrationRequest::from_published_passport(
        &passport,
        operator(),
        RegistrationGranularity::Item,
    )
    .expect("the fixture passport carries all three");

    let mut payloads: Vec<RegistrationPayload> = (0..99).map(|_| payload_from(&request)).collect();
    let mut last = payload_from(&request);
    last.item_id = None;
    payloads.push(last);

    let submission = RegistrationSubmission::new(payloads).expect("exactly at the cap");
    assert_eq!(submission.len(), 100);

    assert!(
        matches!(
            submission.validate(),
            Err(RegistryValidationError::SubmissionPassportInvalid { index, .. }) if index == 99
        ),
        "the hundredth passport must refuse the whole submission: {:?}",
        submission.validate()
    );
}

/// The receipt closes the loop: one Art. 8(8) identifier per passport, in
/// submission order.
#[test]
fn a_successful_submission_is_acknowledged_per_passport() {
    use dpp_registry::{SubmissionOutcome, SubmissionReceipt};

    let identifier = SchemeIdentifier::gs1(Gtin::parse("09506000134352").unwrap());
    let passport = published_passport(identifier.clone());
    let request = RegistrationRequest::from_published_passport(
        &passport,
        operator(),
        RegistrationGranularity::Item,
    )
    .expect("the fixture passport carries all three");
    let submission = RegistrationSubmission::new((0..3).map(|_| payload_from(&request)).collect())
        .expect("within bounds");
    assert!(submission.validate().is_ok());

    let receipt = SubmissionReceipt {
        correlation_id: "corr-8f31".into(),
        outcome: SubmissionOutcome::Success,
        registration_identifiers: vec!["EU-DPP-1".into(), "EU-DPP-2".into(), "EU-DPP-3".into()],
    };

    assert!(receipt.outcome.is_terminal());
    assert_eq!(
        receipt.registration_identifiers.len(),
        submission.len(),
        "Art. 8(10) communicates an identifier for each specific product"
    );
}
/// 🚨 What this composition first exposed, now the other way round.
///
/// This test used to assert that a passport with no operator identifier still
/// produced a request — with `""` in the field — and travelled all the way to
/// `InvalidOperatorId { scheme: "vat", value: "" }`, an error naming the scheme
/// it was given rather than the absence it was not. Three of this file's cases
/// failed on that when it was written, which is how the defect was found: every
/// unit test in both crates passed throughout.
///
/// The constructor now refuses, so the composition asserts the refusal instead.
/// The value of keeping it here rather than only in `dpp-domain` is that this is
/// the seam where the empty string used to escape — the registry crate would
/// have accepted it under any scheme its validator does not recognise.
#[test]
fn a_passport_missing_its_operator_identifier_cannot_be_registered() {
    let identifier = SchemeIdentifier::did("did:web:ecotextile.de:p:1").unwrap();
    let mut passport = published_passport(identifier);
    passport.operator_identifier = None;

    let refused = RegistrationRequest::from_published_passport(
        &passport,
        operator(),
        RegistrationGranularity::Item,
    )
    .expect_err("a registration cannot name an operator the passport never carried");

    assert_eq!(refused.errors.len(), 1);
    assert_eq!(refused.errors[0].field, "/operatorIdentifier");
}
