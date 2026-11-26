//! Helpers the private JWK tests share.

use serde_json::Value;

use crate::jws::algorithm::KeyAlgorithm;

pub(super) const B64: base64::engine::general_purpose::GeneralPurpose =
    base64::engine::general_purpose::URL_SAFE_NO_PAD;

/// The four key files, in the suite's order, with the algorithm each names.
pub(super) const SUITE: [(&str, KeyAlgorithm); 4] = [
    ("p256", KeyAlgorithm::EcdsaP256),
    ("p384", KeyAlgorithm::EcdsaP384),
    ("p521", KeyAlgorithm::EcdsaP521),
    ("ed25519", KeyAlgorithm::Ed25519),
];

/// The verification method of one key file: `p256`, `p384`, `p521` or `ed25519`.
pub(super) fn method(name: &str) -> Value {
    let text = match name {
        "p256" => include_str!("../../../jws/fixtures/w3c-vc-jose-cose/vm-p256.json"),
        "p384" => include_str!("../../../jws/fixtures/w3c-vc-jose-cose/vm-p384.json"),
        "p521" => include_str!("../../../jws/fixtures/w3c-vc-jose-cose/vm-p521.json"),
        "ed25519" => include_str!("../../../jws/fixtures/w3c-vc-jose-cose/vm-ed25519.json"),
        other => panic!("{other} is not one of the suite's key files"),
    };
    serde_json::from_str(text).expect("a verification method")
}

/// The `secretKeyJwk` of one key file, as a copy the test may change.
pub(super) fn secret(name: &str) -> Value {
    method(name)["secretKeyJwk"].clone()
}

/// The `publicKeyJwk` of one key file.
pub(super) fn public(name: &str) -> Value {
    method(name)["publicKeyJwk"].clone()
}
