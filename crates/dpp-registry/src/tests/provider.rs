//! Back-up providers and operator references.

use super::super::*;
use super::common::*;

/// 🚨 A declared back-up link with nobody named is refused.
///
/// ESPR Art. 10(4) is unconditional — a back-up copy is made available *through*
/// a digital product passport service provider — so a payload that declares the
/// link has, as a matter of law, a provider to reference. Annex III(l) is the
/// point that says who, and IR (EU) 2026/1778 Art. 8(9)(c) is the registration
/// data the Commission stores. Sending the location and omitting the party drops
/// a data point the declaration itself proves exists.
#[test]
fn a_backup_url_without_a_provider_is_refused() {
    let mut payload = sample_payload();
    payload.backup_url = Some("https://backup.example.com/dpp/abc.json".into());
    payload.service_provider = None;

    assert!(
        matches!(
            payload.validate(),
            Err(RegistryValidationError::MissingRequiredField(ref f)) if f == "serviceProvider"
        ),
        "a back-up link with no provider named was accepted"
    );
}

/// And not the converse. Art. 8(7)(e) confirms the link *"where relevant"*, so a
/// node may hold the provider relationship without publishing the URL to the
/// registry — which is also the case where a back-up exists and the deployment
/// simply has not declared it here.
#[test]
fn a_provider_without_a_backup_url_is_accepted() {
    let mut payload = sample_payload();
    payload.backup_url = None;
    payload.service_provider = Some(ServiceProviderReference::named("Example Backup GmbH"));

    assert!(payload.validate().is_ok(), "{:?}", payload.validate());
}

/// Neither is a payload that declares no back-up at all.
#[test]
fn declaring_no_backup_needs_no_provider() {
    let mut payload = sample_payload();
    payload.backup_url = None;
    payload.service_provider = None;

    assert!(payload.validate().is_ok(), "{:?}", payload.validate());
}

/// A name is the floor, and it is a real floor: Annex III(l) mandates no
/// identifier scheme for point (l), so a named provider with nothing else is
/// lawful rather than a degraded reference.
#[test]
fn a_name_alone_is_a_complete_reference() {
    assert!(
        ServiceProviderReference::named("Example Backup GmbH")
            .validate()
            .is_ok()
    );
    assert!(matches!(
        ServiceProviderReference::named("   ").validate(),
        Err(RegistryValidationError::MissingRequiredField(ref f)) if f == "serviceProvider.name"
    ));
}

/// 🚨 A scheme and a value arrive together or not at all.
///
/// The reason `OperatorIdentifier::validate` already gives: a value with no
/// scheme does not say whether it is a VAT number, an LEI or a DID, so it
/// identifies nobody while looking as though it does. A scheme with no value is
/// the same defect mirrored — and both would otherwise pass, because the
/// per-scheme check accepts any unrecognised scheme including the empty one.
#[test]
fn a_half_stated_identifier_is_refused() {
    let named = |scheme: Option<&str>, value: Option<&str>| ServiceProviderReference {
        name: "Example Backup GmbH".into(),
        scheme: scheme.map(Into::into),
        value: value.map(Into::into),
        country: None,
    };

    for (scheme, value, expected) in [
        (Some("lei"), None, "serviceProvider.value"),
        (None, Some("529900T8BM49AURSDO55"), "serviceProvider.scheme"),
        (Some("  "), Some("anything"), "serviceProvider.scheme"),
        // 🚨 A blank value under a scheme nothing structurally checks.
        // `validate_operator_scheme` accepts `"did"` and every unrecognised
        // scheme without looking at the value, so these reached `Ok` — and
        // `Some("did")` with an empty value is the same absence as
        // `Some("did")` with no value, which the first case above refuses.
        // Two answers for one state is the defect, not the leniency.
        (Some("did"), Some(""), "serviceProvider.value"),
        (Some("did"), Some("   "), "serviceProvider.value"),
        (Some("something-new"), Some(""), "serviceProvider.value"),
    ] {
        assert!(
            matches!(
                named(scheme, value).validate(),
                Err(RegistryValidationError::MissingRequiredField(ref f)) if f == expected
            ),
            "{scheme:?}/{value:?} should have been refused as {expected}"
        );
    }

    // Stated in full, it goes through the same per-scheme check an operator
    // identifier does — a provider is a legal person identified the same way.
    assert!(
        named(Some("lei"), Some("529900T8BM49AURSDO55"))
            .validate()
            .is_ok()
    );
    assert!(named(Some("lei"), Some("not-an-lei")).validate().is_err());
}

/// The country is validated when present and optional when not — Annex III(l)
/// asks for a reference, not an establishment record.
#[test]
fn a_provider_country_is_optional_and_checked_when_given() {
    let with_country = |country: &str| ServiceProviderReference {
        country: Some(country.into()),
        ..ServiceProviderReference::named("Example Backup GmbH")
    };

    assert!(with_country("DE").validate().is_ok());
    assert!(matches!(
        with_country("Germany").validate(),
        Err(RegistryValidationError::InvalidCountryCode { .. })
    ));
}

// ── EN 18219 clause 5 → registry scheme ──────────────────────────────────────
