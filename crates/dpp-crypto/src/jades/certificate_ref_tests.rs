//! The `x5c` and `x5t#S256` pair a JAdES header carries, checked where the
//! header is written: the digest must be the one of the first certificate in the
//! chain, or the header would reference one certificate and carry another.
//!
//! The chain entries here are stand-ins, not parsed certificates. The rule is
//! about which bytes are digested, so a fixed blob is enough.

use base64::Engine;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use super::tests::FAKE_DER;
use super::*;

/// A second stand-in certificate, for a chain with more than one entry.
const INTERMEDIATE_DER: &[u8] = b"\x30\x82\x02\x0a-not-a-real-intermediate-";

/// The `x5t#S256` for `der`, from the one function that computes it.
fn thumbprint_of(der: &[u8]) -> String {
    let CertificateRef::Thumbprint(t) = CertificateRef::thumbprint_of_der(der) else {
        unreachable!("thumbprint_of_der always yields a Thumbprint");
    };
    t
}

fn header_with(certificate: CertificateRef) -> JadesHeader {
    JadesHeader {
        alg: "EdDSA".into(),
        iat: 1_770_000_000,
        certificate,
        content_type: None,
    }
}

/// The `x5c` and the digest `chain_of_der` derives from two certificates. The
/// digest is the first certificate's.
fn two_certificate_chain() -> (Vec<String>, String) {
    let certificates = [FAKE_DER.to_vec(), INTERMEDIATE_DER.to_vec()];
    let CertificateRef::ChainWithThumbprint { chain, thumbprint } =
        CertificateRef::chain_of_der(&certificates).expect("two certificates")
    else {
        unreachable!("chain_of_der yields a ChainWithThumbprint");
    };
    (chain, thumbprint)
}

/// A chain entry that is not base64 has no digest a header can be checked
/// against, so it is refused as a mismatch rather than written.
#[test]
fn a_chain_entry_that_is_not_base64_is_a_mismatch() {
    let reference = CertificateRef::ChainWithThumbprint {
        chain: vec!["!!not-base64!!".into()],
        thumbprint: thumbprint_of(FAKE_DER),
    };
    assert_eq!(
        header_with(reference)
            .to_json_bytes()
            .expect_err("a chain entry that is not base64 is refused"),
        JadesError::ThumbprintMismatch
    );
}

/// A pair built by `chain_of_der` is consistent by construction, so it
/// serialises. The `x5t#S256` written is the digest of the first certificate,
/// computed here independently rather than read back from the constructor.
#[test]
fn a_consistent_pair_serialises_with_the_digest_of_the_first_certificate() {
    let reference = CertificateRef::chain_of_der(&[FAKE_DER.to_vec()]).expect("one certificate");
    let bytes = header_with(reference)
        .to_json_bytes()
        .expect("a consistent pair serialises");
    let written: Map<String, Value> =
        serde_json::from_slice(&bytes).expect("the protected header is a JSON object");

    assert_eq!(
        written["x5t#S256"],
        Value::String(
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(Sha256::digest(FAKE_DER))
        )
    );
}

/// The thumbprint is taken over `chain[0]` alone, so a longer chain whose
/// digest is its first certificate's serialises.
#[test]
fn a_longer_chain_serialises_when_the_digest_is_its_first_certificates() {
    let (chain, thumbprint) = two_certificate_chain();
    header_with(CertificateRef::ChainWithThumbprint { chain, thumbprint })
        .to_json_bytes()
        .expect("the digest is that of chain[0]");
}

/// The same chain with the digest of a later certificate is the mismatch the
/// check exists for, and is refused.
#[test]
fn a_longer_chain_with_the_digest_of_a_later_certificate_is_refused() {
    let (chain, _) = two_certificate_chain();
    let reference = CertificateRef::ChainWithThumbprint {
        chain,
        thumbprint: thumbprint_of(INTERMEDIATE_DER),
    };
    assert_eq!(
        header_with(reference)
            .to_json_bytes()
            .expect_err("the digest of chain[1] is not the digest of chain[0]"),
        JadesError::ThumbprintMismatch
    );
}
