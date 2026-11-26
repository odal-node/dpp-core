//! The registration path, idempotency and identifier limits the registry was observed to apply.

use super::super::*;
use super::common::*;
use chrono::Utc;
use uuid::Uuid;

/// The registration path was `/registrations`, which was invented. The client
/// names a different one, so the old value is not merely unverified — it is
/// wrong, and this is the guard against it drifting back.
#[test]
fn the_registration_path_is_the_observed_one() {
    assert_eq!(REGISTRATION_PATH, "/dpp-registration-requests");
    assert_ne!(REGISTRATION_PATH, "/registrations");
}

/// Idempotency is a header, not an envelope field. Both halves matter: the
/// header must be spelled as the registry spells it, and `request_id` must not
/// be mistaken for it again.
#[test]
fn idempotency_is_a_header_and_request_id_is_not_it() {
    assert_eq!(IDEMPOTENCY_KEY_HEADER, "Idempotency-Key");

    let envelope = EuRegistryEnvelope {
        api_version: "1.0".into(),
        request_id: Uuid::now_v7(),
        timestamp: Utc::now(),
        submission: RegistrationSubmission::single(sample_payload()),
    };
    let json = serde_json::to_value(&envelope).unwrap();
    // It travels as ordinary payload under our own name, and de-duplicates
    // nothing at the registry.
    assert!(json.get("requestId").is_some());
    assert!(json.get("idempotencyKey").is_none());
}

/// The 409 the web client recognises. A caller seeing it should read the
/// original submission's outcome rather than resubmit, so recognising it has to
/// be reliable.
#[test]
fn a_replayed_idempotency_key_is_recognisable() {
    let body: RegistryErrorBody = serde_json::from_value(serde_json::json!({
        "subCode": "CONFLICT_IDEMPOTENCY_KEY_ALREADY_USED",
        "traceId": "abc123",
    }))
    .unwrap();

    assert!(body.is_idempotency_key_reused());
    assert_eq!(body.trace_id.as_deref(), Some("abc123"));

    let other: RegistryErrorBody = serde_json::from_value(serde_json::json!({
        "subCode": "SOMETHING_ELSE",
    }))
    .unwrap();
    assert!(!other.is_idempotency_key_reused());

    // An error body carrying neither field is still readable — the shape beyond
    // these two is unknown, so absence must not be a parse failure.
    let bare: RegistryErrorBody = serde_json::from_value(serde_json::json!({})).unwrap();
    assert!(!bare.is_idempotency_key_reused());
    assert!(bare.trace_id.is_none());
}

/// The number that reversed a design risk. A Digital Link of the shape this
/// workspace builds must fit with room to spare — that is the whole point of
/// the constant, and a regression to 50 would fail here rather than at the
/// registry.
#[test]
fn a_digital_link_carrier_url_fits_the_identifier_limit() {
    assert_eq!(MAX_PRODUCT_IDENTIFIER_CHARS, 2000);

    let payload = sample_payload();
    // The real shape: host + GTIN-14 + a 20-character serial. The exact count
    // moves with the host, so what is asserted is the order of magnitude and
    // the headroom — not a figure that a longer hostname would falsify.
    let carrier = "https://id.example.com/01/09506000134352/21/ABCDEFGHIJKLMNOPQRST";
    assert_eq!(carrier.chars().count(), 64);
    assert!(carrier.chars().count() * 30 < MAX_PRODUCT_IDENTIFIER_CHARS);

    let mut fits = payload.clone();
    fits.digital_link_url = carrier.into();
    assert!(fits.validate().is_ok());
}

#[test]
fn an_over_long_product_identifier_is_refused() {
    let mut payload = sample_payload();
    payload.digital_link_url = format!(
        "https://id.example.com/01/{}",
        "0".repeat(MAX_PRODUCT_IDENTIFIER_CHARS)
    );
    let chars = payload.digital_link_url.chars().count();

    assert_eq!(
        payload.validate(),
        Err(RegistryValidationError::ProductIdentifierTooLong {
            chars,
            max: MAX_PRODUCT_IDENTIFIER_CHARS,
        })
    );
}

/// Counted in characters, not bytes. An internationalised host is not longer
/// than the registry thinks it is.
#[test]
fn the_identifier_limit_counts_characters_not_bytes() {
    let mut payload = sample_payload();
    // 1999 characters, of which many are multi-byte — over the limit in bytes,
    // under it in chars.
    payload.digital_link_url = format!("https://é.example/{}", "é".repeat(1981));
    assert_eq!(payload.digital_link_url.chars().count(), 1999);
    assert!(payload.digital_link_url.len() > MAX_PRODUCT_IDENTIFIER_CHARS);
    assert!(payload.validate().is_ok());
}

/// 👁️ The registry takes the identifier only as an `https://` URL, and refuses
/// one naming a non-permitted port (User Guide v1.03 chapter 6.2). Which ports
/// are permitted is unstated, so only the default one is let through.
#[test]
fn the_identifier_must_be_an_https_url_on_the_default_port() {
    let with_url = |url: &str| RegistrationPayload {
        digital_link_url: url.into(),
        ..sample_payload()
    };
    let refused = |url: &str| match with_url(url).validate() {
        Err(RegistryValidationError::InvalidProductIdentifierUrl { value, reason }) => {
            assert_eq!(value, url);
            reason
        }
        other => panic!("{url} should be refused as a URL, got {other:?}"),
    };

    assert_eq!(
        refused("http://id.example.com/01/09506000134352"),
        "must start with https://"
    );
    assert_eq!(
        refused("did:web:id.example.com:p:1"),
        "must start with https://"
    );
    assert_eq!(refused("https:///01/09506000134352"), "names no host");
    assert_eq!(
        refused("https://id.example.com:8443/01/09506000134352"),
        "names a port other than 443"
    );
    assert_eq!(
        refused("https://[2001:db8::1]:8443/01/09506000134352"),
        "names a port other than 443"
    );

    for accepted in [
        "https://id.example.com/01/09506000134352",
        "https://id.example.com:443/01/09506000134352",
        "https://[2001:db8::1]/01/09506000134352",
        "https://id.example.com?x=1",
    ] {
        assert!(
            with_url(accepted).validate().is_ok(),
            "{accepted} should be accepted: {:?}",
            with_url(accepted).validate()
        );
    }
}
