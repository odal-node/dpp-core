//! The certificate chain, its digest and thumbprint.

use base64::Engine;

use super::super::*;
use super::common::*;

/// A signature over the *digest* of the signing input is the remote-signing
/// hand-off, and it commits to the same bytes.
///
/// This is the operation a provider's "sign this hash" endpoint performs, and
/// the test exists to pin that the digest is taken over the signing input —
/// not over the payload, which is the easy and silent mistake.
#[test]
fn the_digest_helper_commits_to_the_signing_input() {
    use sha2::{Digest, Sha256};
    let prepared = prepare(&header(), b"payload").expect("prepares");
    let expected: [u8; 32] = Sha256::digest(prepared.signing_input()).into();
    assert_eq!(prepared.signing_input_sha256(), expected);

    // And explicitly *not* a digest of the payload alone, which would sign
    // something that omits the header — including the certificate reference.
    let payload_only: [u8; 32] = Sha256::digest(b"payload").into();
    assert_ne!(
        prepared.signing_input_sha256(),
        payload_only,
        "the signature must cover the header, not only the payload"
    );
}

/// A missing certificate reference is refused, not silently omitted.
///
/// Clause 5.1.7 makes one of `x5t#S256`/`x5c`/`sigX5ts`/`x5t#o` mandatory. A
/// signature without one is not a JAdES signature, so producing one would be
/// producing something mislabelled.
#[test]
fn an_empty_certificate_chain_is_refused() {
    let h = JadesHeader {
        certificate: CertificateRef::Chain(vec![]),
        ..header()
    };
    assert_eq!(
        prepare(&h, b"payload").unwrap_err(),
        JadesError::EmptyCertificateChain
    );
}

/// `x5c` carries the chain as a JSON array, signing certificate first.
#[test]
fn x5c_carries_the_chain_in_order() {
    let h = JadesHeader {
        certificate: CertificateRef::Chain(vec!["c3ViamVjdA==".into(), "aXNzdWVy".into()]),
        ..header()
    };
    let prepared = prepare(&h, b"payload").expect("prepares");
    let decoded = decode_header(prepared.assemble(&[0u8; 64]).as_str());
    assert_eq!(
        decoded["x5c"],
        serde_json::json!(["c3ViamVjdA==", "aXNzdWVy"]),
        "x5c is an ordered array, signing certificate first (RFC 7515 4.1.6)"
    );
    assert!(
        decoded.get("x5t#S256").is_none(),
        "one reference form, not both, unless deliberately migrating"
    );
}

/// `x5t#S256` is the base64url SHA-256 of the DER certificate.
///
/// RFC 7515 clause 4.1.8. Checked against an independently computed digest
/// rather than against whatever the constructor happened to produce.
#[test]
fn the_thumbprint_is_the_sha256_of_the_der() {
    use sha2::{Digest, Sha256};
    let CertificateRef::Thumbprint(t) = CertificateRef::thumbprint_of_der(FAKE_DER) else {
        panic!("constructed a thumbprint");
    };
    assert_eq!(t, B64.encode(Sha256::digest(FAKE_DER)));
    assert!(!t.contains('='), "base64url without padding");
}

/// `cty` appears only when set.
#[test]
fn content_type_is_optional_and_emitted_when_present() {
    let plain = prepare(&header(), b"p").expect("prepares");
    assert!(
        decode_header(plain.assemble(&[0u8; 64]).as_str())
            .get("cty")
            .is_none()
    );

    let typed = prepare(&header().with_content_type("json"), b"p").expect("prepares");
    assert_eq!(
        decode_header(typed.assemble(&[0u8; 64]).as_str())["cty"],
        "json"
    );
}

/// The header bytes are stable across calls.
///
/// They are signed, so two serialisations that differ only in key order are two
/// different signing inputs. A verifier recomputing the header from a
/// round-tripped structure would then disagree with the signature for no
/// reason a reader could see.
#[test]
fn header_serialisation_is_stable() {
    let h = header().with_content_type("json");
    let once = h.to_json_bytes().expect("serialises");
    for _ in 0..64 {
        assert_eq!(h.to_json_bytes().expect("serialises"), once);
    }
}

// ─── Table 1 conformance, mechanically ───────────────────────────────────────
