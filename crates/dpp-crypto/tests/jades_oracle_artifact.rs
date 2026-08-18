//! Emit a real JAdES-B-B artefact for an external validator to judge.
//!
//! # Why this exists
//!
//! Every other test of the JAdES module checks our output against **our own
//! reading** of ETSI TS 119 182-1. That reading is careful and every clause is
//! cited, but it is still ours: a test built from a transcription agrees with
//! the transcription, including where the transcription is wrong.
//!
//! The European Commission publishes the reference implementation of AdES
//! creation and validation (DSS). Handing it an artefact and asking *"what is
//! this?"* is the only check available here that is not circular. This test
//! produces the artefact; `.github/oracle/jades/` runs the validator.
//!
//! # Why the certificate is self-signed, and why that is enough
//!
//! The question being asked is **"is this structurally a JAdES-BASELINE-B
//! signature?"**, not "is this a qualified seal". The second is a question about
//! a certificate's issuer, a creation device and a supervised trust service —
//! none of which a test can conjure, and all of which are exactly what the
//! module documentation says this code does not provide.
//!
//! A self-signed certificate parses, carries a public key, and lets a validator
//! determine the signature form and level. It will also, correctly, report the
//! signature as untrusted. That is the expected and honest outcome: the oracle
//! asserts the **form and level**, and asserts nothing about trust.
//!
//! # Running
//!
//! ```text
//! EMIT_JADES_ARTIFACT=1 cargo test -p dpp-crypto --test jades_oracle_artifact
//! ```
//!
//! Writes to `target/jades-oracle/`. Without the variable the test still runs
//! and still checks the artefact is well-formed — it simply does not write it,
//! so an ordinary `just check` neither produces files nor skips the coverage.

use base64::Engine;
use dpp_crypto::jades::{CertificateRef, JadesHeader, prepare};
use ed25519_dalek::{Signer, SigningKey};

const B64URL: base64::engine::general_purpose::GeneralPurpose =
    base64::engine::general_purpose::URL_SAFE_NO_PAD;
const B64STD: base64::engine::general_purpose::GeneralPurpose =
    base64::engine::general_purpose::STANDARD;

/// The PKCS#8 v1 prefix for an Ed25519 private key, per RFC 8410 clause 7.
///
/// Fixed: version 0, the `id-Ed25519` algorithm identifier (OID 1.3.101.112),
/// then the 32-byte seed inside an OCTET STRING inside an OCTET STRING. Used to
/// hand one key to two libraries — `ed25519-dalek` signs with it, `rcgen`
/// certifies it — so the certificate genuinely attests the key that signed.
///
/// The construction is checked rather than trusted: the test asserts the
/// certificate's embedded public key equals the signing key's own.
const PKCS8_ED25519_PREFIX: [u8; 16] = [
    0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04, 0x20,
];

fn pkcs8_der(signing_key: &SigningKey) -> Vec<u8> {
    let mut der = Vec::with_capacity(48);
    der.extend_from_slice(&PKCS8_ED25519_PREFIX);
    der.extend_from_slice(&signing_key.to_bytes());
    der
}

/// The payload an artefact carries: a passport-shaped document, so the oracle
/// judges something the same shape as the real thing.
fn payload() -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "passportId": "01937f4e-0000-7000-8000-000000000001",
        "sector": "battery",
        "schemaVersion": "2.6.0",
        "gtin": "09506000134352",
        "status": "published",
    }))
    .expect("payload serialises")
}

#[test]
fn emit_and_self_check_a_jades_artifact() {
    // One Ed25519 key, used twice: dalek signs the JWS, rcgen certifies the
    // same key. A certificate over a different key would make the artefact
    // structurally valid and semantically a lie.
    // `dpp-crypto`'s internal `os_rng()` is crate-private, so an integration
    // test reaches for the same underlying source directly.
    let mut rng = rand::rand_core::UnwrapErr(rand::rngs::SysRng);
    let signing_key = SigningKey::generate(&mut rng);
    let key_pair = rcgen::KeyPair::try_from(pkcs8_der(&signing_key).as_slice())
        .expect("rcgen accepts the PKCS#8 encoding");

    let mut params = rcgen::CertificateParams::new(vec!["jades-oracle.test.invalid".to_owned()])
        .expect("subject alt name is valid");
    params
        .distinguished_name
        .push(rcgen::DnType::CommonName, "Odal Node JAdES oracle (test)");
    params
        .distinguished_name
        .push(rcgen::DnType::OrganizationName, "Odal Node");
    let certificate = params
        .self_signed(&key_pair)
        .expect("a self-signed certificate is issuable");
    let cert_der = certificate.der().to_vec();

    // The construction above is a hand-built PKCS#8. Prove the two libraries
    // ended up on the same key rather than assuming it.
    let cert_spki = x509_public_key(&cert_der);
    assert_eq!(
        cert_spki,
        signing_key.verifying_key().to_bytes(),
        "the certificate must attest the key that signs, or the artefact is a lie"
    );

    // Build and sign.
    let header = JadesHeader {
        alg: "EdDSA".into(),
        iat: chrono::Utc::now().timestamp(),
        certificate: CertificateRef::chain_of_der(&[cert_der.clone()])
            .expect("a one-certificate chain"),
        content_type: Some("json".into()),
    };
    let body = payload();
    let prepared = prepare(&header, &body).expect("prepares");
    let signature = signing_key.sign(prepared.signing_input());
    let compact = prepared.assemble(&signature.to_bytes()).into_string();

    // Self-check before handing it to anyone: three segments, the payload comes
    // back unchanged, and the signature verifies against the certified key.
    let parts: Vec<&str> = compact.split('.').collect();
    assert_eq!(parts.len(), 3);
    assert_eq!(B64URL.decode(parts[1]).expect("payload b64"), body);
    {
        use ed25519_dalek::Verifier;
        let sig: [u8; 64] = B64URL
            .decode(parts[2])
            .expect("signature b64")
            .try_into()
            .expect("64 bytes");
        signing_key
            .verifying_key()
            .verify(
                format!("{}.{}", parts[0], parts[1]).as_bytes(),
                &ed25519_dalek::Signature::from_bytes(&sig),
            )
            .expect("the emitted artefact verifies as a plain JWS");
    }

    if std::env::var("EMIT_JADES_ARTIFACT").is_err() {
        return;
    }

    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/jades-oracle")
        .canonicalize()
        .unwrap_or_else(|_| {
            let p =
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/jades-oracle");
            std::fs::create_dir_all(&p).expect("artefact directory is writable");
            p
        });
    std::fs::create_dir_all(&dir).expect("artefact directory is writable");
    std::fs::write(dir.join("signature.jws"), compact.as_bytes()).expect("writes the signature");
    std::fs::write(dir.join("certificate.der"), &cert_der).expect("writes the certificate");
    println!("wrote JAdES artefact to {}", dir.display());
}

/// Pull the 32-byte Ed25519 public key out of a DER certificate's
/// SubjectPublicKeyInfo.
///
/// Uses `x509-parser`, which `rcgen` already brings in, rather than
/// index-arithmetic on DER — the point of this helper is to *check* the
/// hand-built PKCS#8, and checking it with another hand-rolled parser would
/// prove nothing.
fn x509_public_key(der: &[u8]) -> [u8; 32] {
    use x509_parser::prelude::*;
    let (_, cert) = X509Certificate::from_der(der).expect("certificate parses");
    cert.public_key()
        .subject_public_key
        .data
        .as_ref()
        .try_into()
        .expect("an Ed25519 public key is 32 bytes")
}
