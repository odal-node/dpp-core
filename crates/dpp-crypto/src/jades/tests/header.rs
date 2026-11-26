//! JAdES-B-B construction tests.
//!
//! Every assertion here is traceable to a clause of ETSI TS 119 182-1 V1.2.1,
//! and the clause is named. A test that pins a format against nothing but our
//! own reading of it is a test that agrees with us; naming the clause is what
//! lets the next reader check the reading rather than the code.

use base64::Engine;
use ed25519_dalek::{Signer, SigningKey, Verifier};

use super::super::*;
use super::common::*;

/// The header carries `alg`, a certificate reference and `iat` — and nothing
/// the standard does not ask for at B-B.
///
/// Table 1: `alg` shall be present; the claimed-signing-time service shall be
/// provided; the signing-certificate-reference service has cardinality 1.
#[test]
fn a_b_b_header_carries_exactly_what_the_standard_requires() {
    let prepared = prepare(&header(), b"payload").expect("prepares");
    let signed = prepared.assemble(&[0u8; 64]);
    let h = decode_header(signed.as_str());

    assert_eq!(h["alg"], "EdDSA");
    assert!(
        h.get("x5t#S256").is_some(),
        "clause 5.1.7 requires a reference"
    );
    assert!(h.get("iat").is_some(), "clause 5.1.11 requires iat");

    // Sorted, because `serde_json::Map` is a `BTreeMap` without the
    // `preserve_order` feature. JWS places no constraint on header key order,
    // and the property that matters — that the assembled bytes are the signed
    // bytes — comes from retaining the encoded segment, not from the ordering.
    let mut keys: Vec<&str> = h
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        vec!["alg", "iat", "x5t#S256"],
        "no parameter the standard does not ask for at B-B"
    );
}

/// `iat` is an integer number of seconds.
///
/// Clause 5.1.11: the value *"shall be an integer number"* and *"shall not
/// contain fractions of seconds"*. A float here would be a conformance defect
/// that no signature check would ever catch.
#[test]
fn iat_is_a_whole_number_of_seconds() {
    let prepared = prepare(&header(), b"payload").expect("prepares");
    let h = decode_header(prepared.assemble(&[0u8; 64]).as_str());
    let iat = &h["iat"];
    assert!(iat.is_i64(), "iat must be an integer, got {iat}");
    assert_eq!(iat.as_i64(), Some(1_770_000_000));
    assert!(
        !serde_json::to_string(iat)
            .expect("serialises")
            .contains('.'),
        "iat must carry no fractional part"
    );
}

/// No `crit`, and no `sigD`, for an attached payload.
///
/// Clause 5.2.8.1: `sigD` *"shall not appear in JAdES signatures whose JWS
/// Payload is attached"*. Clause 5.1.9: `crit` is required only when `sigD` is
/// present — V1.2.1 **suppressed** V1.1.1's blanket requirement precisely so
/// that a signature without `sigD` stays processable by a plain JWS library.
///
/// This test is the one that would fail if someone "improved" the header by
/// adding `crit` from memory of the older version.
#[test]
fn an_attached_payload_emits_neither_sigd_nor_crit() {
    let prepared = prepare(&header(), b"payload").expect("prepares");
    let h = decode_header(prepared.assemble(&[0u8; 64]).as_str());
    assert!(
        h.get("sigD").is_none(),
        "clause 5.2.8.1 forbids sigD when attached"
    );
    assert!(
        h.get("crit").is_none(),
        "clause 5.1.9 requires crit only alongside sigD; V1.2.1 NOTE 1 explains why"
    );
}

/// The result is a plain RFC 7515 compact JWS, verifiable as one.
///
/// The payoff of the clause above: one artefact serves a verifier that
/// understands AdES and one that only understands JWS. Here the whole
/// round-trip runs through an ordinary Ed25519 verify.
#[test]
fn the_output_verifies_as_an_ordinary_jws() {
    let key = SigningKey::generate(&mut crate::os_rng());
    let payload = br#"{"passportId":"abc","productGroup":"battery"}"#;

    let prepared = prepare(&header(), payload).expect("prepares");
    let signature = key.sign(prepared.signing_input());
    let compact = prepared.assemble(&signature.to_bytes()).into_string();

    let parts: Vec<&str> = compact.split('.').collect();
    assert_eq!(parts.len(), 3, "compact serialisation is three segments");

    // Verify exactly as a plain JWS consumer would: recompute the signing input
    // from the wire, do not trust what we kept in memory.
    let signing_input = format!("{}.{}", parts[0], parts[1]);
    let sig_bytes: [u8; 64] = B64
        .decode(parts[2])
        .expect("signature is base64url")
        .try_into()
        .expect("Ed25519 signature is 64 bytes");
    key.verifying_key()
        .verify(
            signing_input.as_bytes(),
            &ed25519_dalek::Signature::from_bytes(&sig_bytes),
        )
        .expect("a JAdES-B-B signature verifies as a plain JWS");

    assert_eq!(
        B64.decode(parts[1]).expect("payload is base64url"),
        payload,
        "the payload round-trips unchanged"
    );
}

/// Tampering with the payload breaks the signature.
///
/// Content binding is the whole point; a format test that never checks it has
/// only tested the packaging.
#[test]
fn a_tampered_payload_no_longer_verifies() {
    let key = SigningKey::generate(&mut crate::os_rng());
    let prepared = prepare(&header(), b"original").expect("prepares");
    let signature = key.sign(prepared.signing_input());
    let compact = prepared.assemble(&signature.to_bytes()).into_string();

    let parts: Vec<&str> = compact.split('.').collect();
    let tampered = format!("{}.{}", parts[0], B64.encode(b"substituted"));
    let sig_bytes: [u8; 64] = B64.decode(parts[2]).expect("b64").try_into().expect("64");

    assert!(
        key.verifying_key()
            .verify(
                tampered.as_bytes(),
                &ed25519_dalek::Signature::from_bytes(&sig_bytes)
            )
            .is_err(),
        "a substituted payload must not verify"
    );
}
