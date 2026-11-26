//! The signing time, the signature and the combined form.

use base64::Engine;

use super::super::*;
use super::common::*;
use super::table_1::table_1_b_b_violations;

/// `now()` claims a signing time in whole seconds, close to now.
#[test]
fn now_claims_a_plausible_whole_second_signing_time() {
    let before = chrono::Utc::now().timestamp();
    let h = JadesHeader::now("EdDSA", CertificateRef::thumbprint_of_der(FAKE_DER));
    let after = chrono::Utc::now().timestamp();
    assert!(
        (before..=after).contains(&h.iat),
        "iat {} is outside [{before}, {after}]",
        h.iat
    );
    let compact = prepare(&h, b"p").expect("prepares").assemble(&[0u8; 64]);
    assert!(table_1_b_b_violations(compact.as_str()).is_empty());
}

/// The module is indifferent to signature length, because the algorithm is the
/// caller's business.
///
/// `ES256` produces 64 bytes, `RS256` 256, Ed25519 64. Baking in an expectation
/// would silently restrict which providers could be used — the opposite of the
/// point of building the format ourselves.
#[test]
fn signature_length_is_not_constrained() {
    for len in [64usize, 96, 128, 256, 384, 512] {
        let compact = prepare(&header(), b"payload")
            .expect("prepares")
            .assemble(&vec![0xabu8; len])
            .into_string();
        assert!(table_1_b_b_violations(&compact).is_empty(), "len {len}");
        let sig = B64
            .decode(compact.split('.').nth(2).expect("sig segment"))
            .expect("b64");
        assert_eq!(sig.len(), len, "the signature round-trips at {len} bytes");
    }
}

/// An empty payload still produces a well-formed signature.
///
/// `base64url("")` is the empty string, so the compact form has an empty middle
/// segment — legal in JWS, and the kind of thing a naive assembler gets wrong.
#[test]
fn an_empty_payload_produces_an_empty_middle_segment() {
    let compact = prepare(&header(), b"")
        .expect("prepares")
        .assemble(&[0u8; 64])
        .into_string();
    let parts: Vec<&str> = compact.split('.').collect();
    assert_eq!(parts.len(), 3);
    assert!(parts[1].is_empty(), "an empty payload encodes to nothing");
    assert!(table_1_b_b_violations(&compact).is_empty());
}

/// No segment carries base64 padding.
///
/// RFC 7515 clause 2 requires base64url **without** padding. A `=` anywhere is
/// a signature other implementations will reject.
#[test]
fn no_segment_carries_base64_padding() {
    // A payload length that is not a multiple of three, so padding would show
    // if the encoder emitted any.
    let compact = prepare(&header().with_content_type("json"), b"1234567890abcdefghij")
        .expect("prepares")
        .assemble(&[9u8; 64])
        .into_string();
    assert!(
        !compact.contains('='),
        "base64url in JWS is unpadded (RFC 7515 clause 2): {compact}"
    );
}

/// The combined form emits both `x5c` and `x5t#S256`.
///
/// Clause 5.1.7 permits either alone; Table 1's "signing a reference of the
/// signing certificate" service admits only the digest forms. A signature with
/// `x5c` alone therefore satisfies 5.1.7 and is still not baseline — the
/// European Commission's DSS reported exactly that as `JSON-NOT-ETSI`, warning
/// that the signing-certificate attribute was absent.
///
/// Found by an outside implementation rather than by reading, which is the
/// whole reason the oracle exists.
#[test]
fn the_combined_form_carries_the_chain_and_the_digest() {
    let der = FAKE_DER.to_vec();
    let cert = CertificateRef::chain_of_der(std::slice::from_ref(&der)).expect("one certificate");
    let h = JadesHeader {
        certificate: cert,
        ..header()
    };
    let decoded = decode_header(
        prepare(&h, b"payload")
            .expect("prepares")
            .assemble(&[0u8; 64])
            .as_str(),
    );

    assert!(
        decoded.get("x5c").is_some(),
        "the chain travels with the signature"
    );
    let CertificateRef::Thumbprint(expected) = CertificateRef::thumbprint_of_der(&der) else {
        unreachable!()
    };
    assert_eq!(
        decoded["x5t#S256"],
        serde_json::json!(expected),
        "and the digest reference Table 1 requires for baseline"
    );
    assert!(
        table_1_b_b_violations(
            prepare(&h, b"payload")
                .expect("prepares")
                .assemble(&[0u8; 64])
                .as_str()
        )
        .is_empty()
    );
}

/// An empty chain is refused by the combined constructor too.
#[test]
fn the_combined_constructor_refuses_an_empty_chain() {
    assert_eq!(
        CertificateRef::chain_of_der(&[]).unwrap_err(),
        JadesError::EmptyCertificateChain
    );
}

// ── The EU profile, which lives in a Regulation and not in the standard ──────
