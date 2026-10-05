//! RFC 8037 appendix A's Ed25519 examples, run through this crate's own JWK
//! emitter, key reader and JWS verifier.
//!
//! Unlike the RFC 8032 vectors, which exercise the signature library, these
//! reach code written here: the `publicKeyJwk` that [`KeyAlgorithm`] emits, the
//! key a DID document's JWK is read as, and [`verify_jws`] given a JWS that this
//! crate did not produce. The RFC authored the token and its key, so a
//! verifier that agreed with only its own signer would fail on it.
//!
//! Covered: A.1 and A.2 (the key pair and its public JWK) and A.4 and A.5 (the
//! JWS signing example and its validation). A.4 and A.5 carry `alg: EdDSA`, which
//! is what this crate emits and verifies.
//!
//! A.3, the JWK thumbprint, is checked against the thumbprint code in
//! `thumbprint_tests.rs`. Here it is the identifier the RFC's key is filed under
//! in a DID document, which the key reader holds the key to.
//!
//! The compact JWS is assembled from its three published parts instead of being
//! written as one string: a token-shaped literal in source is what secret
//! scanners are for.

use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};
use serde_json::{Value, json};

use super::algorithm::KeyAlgorithm;
use super::verifier::{resolve_verification_key, verify_jws};

const B64: base64::engine::GeneralPurpose = base64::engine::general_purpose::URL_SAFE_NO_PAD;

/// A.1: the private key's `d`, and the hexadecimal dump the RFC prints with it.
const PRIVATE_D: &str = "nWGxne_9WmC6hEr0kuwsxERJxWl7MmkZcDusAxyuf2A";
const PRIVATE_HEX: &str = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";

/// A.1 and A.2: the public key's `x`, and its hexadecimal dump.
const PUBLIC_X: &str = "11qYAYKxCrfVS_7TyWQHOg7hcvPapiMlrwIaaPcHURo";
const PUBLIC_HEX: &str = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";

/// A.2: the public JWK, as the RFC prints it.
const PUBLIC_JWK: &str = r#"{"kty":"OKP","crv":"Ed25519",
   "x":"11qYAYKxCrfVS_7TyWQHOg7hcvPapiMlrwIaaPcHURo"}"#;

/// A.4: the protected header, the payload text, and each one's base64url form.
const HEADER_JSON: &str = r#"{"alg":"EdDSA"}"#;
const HEADER_B64: &str = "eyJhbGciOiJFZERTQSJ9";
const PAYLOAD_TEXT: &str = "Example of Ed25519 signing";
const PAYLOAD_B64: &str = "RXhhbXBsZSBvZiBFZDI1NTE5IHNpZ25pbmc";

/// A.4: the signature over `header.payload`, in hexadecimal and in base64url.
const SIGNATURE_HEX: &str = "860c98d2297f3060a33f42739672d61b53cf3adefed3d3c672f320dc021b411e9d59b8628dc351e248b88b29468e0e41855b0fb7d83bb15be902bfccb8cd0a02";
const SIGNATURE_B64: &str =
    "hgyY0il_MGCjP0JzlnLWG1PPOt7-09PGcvMg3AIbQR6dWbhijcNR4ki4iylGjg5BhVsPt9g7sVvpAr_MuM0KAg";

fn private_key() -> SigningKey {
    let secret: [u8; 32] = hex::decode(PRIVATE_HEX)
        .expect("hex")
        .try_into()
        .expect("32 bytes");
    SigningKey::from_bytes(&secret)
}

/// A.1: the private key is the same 32 bytes in both of the RFC's forms, and
/// derives the public key the RFC prints. The JWK this crate builds from that
/// public key is the RFC's A.2 JWK, member for member.
#[test]
fn the_rfc_key_pair_yields_the_rfc_public_jwk() {
    assert_eq!(
        B64.decode(PRIVATE_D).expect("base64url"),
        hex::decode(PRIVATE_HEX).expect("hex"),
        "A.1: `d` and its hexadecimal dump are one key"
    );

    let public = private_key().verifying_key().to_bytes();
    assert_eq!(hex::encode(public), PUBLIC_HEX);
    assert_eq!(B64.encode(public), PUBLIC_X);

    let rfc_jwk: Value = serde_json::from_str(PUBLIC_JWK).expect("the RFC's JWK parses");
    assert_eq!(KeyAlgorithm::Ed25519.public_key_jwk(&public), rfc_jwk);
}

/// The other direction: the RFC's public JWK, placed in a DID document the way
/// `did_builder` places this crate's own, is read back as the RFC's `x`.
///
/// The method is filed under the thumbprint A.3 publishes for this key, so the
/// key reader's check that a thumbprint identifier is the key's own is met by
/// a value the RFC computed, not one this crate did.
#[test]
fn the_rfc_public_jwk_is_read_as_a_did_document_key() {
    const DID: &str = "did:web:example";
    let rfc_jwk: Value = serde_json::from_str(PUBLIC_JWK).expect("the RFC's JWK parses");
    let id = format!(
        "{DID}#urn:ietf:params:oauth:jwk-thumbprint:sha-256:kPrK_qmxVWaYVA9wwBF6Iuo3vVzz7TxHCTwXBygrS4k"
    );
    let document = json!({
        "id": DID,
        "verificationMethod": [
            {"id": id, "type": "JsonWebKey", "controller": DID, "publicKeyJwk": rfc_jwk},
        ],
        "assertionMethod": [id],
    });

    // Only the header is read, so the payload and signature are placeholders.
    let header = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(json!({"alg": "EdDSA", "kid": id}).to_string());
    let token = format!("{header}.e30.c2ln");

    assert_eq!(
        resolve_verification_key(&document, &token).as_deref(),
        Some(PUBLIC_X)
    );
}

/// A.4: every intermediate value, and the signature, reproduce.
#[test]
fn the_rfc_signing_example_reproduces_byte_for_byte() {
    assert_eq!(B64.encode(HEADER_JSON), HEADER_B64);
    assert_eq!(B64.encode(PAYLOAD_TEXT), PAYLOAD_B64);

    let signing_input = format!("{HEADER_B64}.{PAYLOAD_B64}");
    let signature = private_key().sign(signing_input.as_bytes()).to_bytes();

    assert_eq!(hex::encode(signature), SIGNATURE_HEX);
    assert_eq!(B64.encode(signature), SIGNATURE_B64);
}

/// A.5: the verifier accepts the JWS the RFC publishes, under the RFC's public
/// key, and refuses it once the payload is changed.
#[test]
fn this_verifier_accepts_the_rfc_jws_and_refuses_an_altered_one() {
    let published = format!("{HEADER_B64}.{PAYLOAD_B64}.{SIGNATURE_B64}");
    assert!(verify_jws(&published, PUBLIC_X).expect("well-formed"));

    let altered_payload = B64.encode(format!("{PAYLOAD_TEXT}!"));
    let altered = format!("{HEADER_B64}.{altered_payload}.{SIGNATURE_B64}");
    assert!(!verify_jws(&altered, PUBLIC_X).expect("well-formed"));
}
