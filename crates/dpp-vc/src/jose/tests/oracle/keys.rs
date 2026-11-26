//! The suite's four keys, which sign everything in the corpus.
//!
//! The W3C suite publishes one key for each algorithm, secret half included, for
//! its own tests. Signing with them makes the corpus reproducible, since this
//! crate's ECDSA is deterministic (RFC 6979), and lets the library sign with the
//! same keys in the other direction.

use serde_json::{Map, Value};

use crate::test_support::{key_file, key_id, public_key, signing_key};
use dpp_crypto::jws::PublicKey;
use dpp_crypto::keystore::KeyEntry;

/// Each algorithm, with the suite file that holds its key.
pub(super) const ALGORITHMS: [(&str, &str); 4] = [
    ("EdDSA", "ed25519"),
    ("ES256", "p256"),
    ("ES384", "p384"),
    ("ES512", "p521"),
];

/// The file of the key that signs under `alg`.
pub(super) fn file_of(alg: &str) -> &'static str {
    ALGORITHMS
        .iter()
        .find(|(name, _)| *name == alg)
        .unwrap_or_else(|| panic!("{alg} is not a JOSE algorithm of this crate"))
        .1
}

pub(super) fn signing(alg: &str) -> KeyEntry {
    signing_key(file_of(alg))
}

pub(super) fn public(alg: &str) -> PublicKey {
    public_key(file_of(alg))
}

/// The `kid` this crate signs under: the absolute URL of the key's verification
/// method, which §4.2 asks for when a credential has no `iss`.
pub(super) fn kid(alg: &str) -> String {
    key_id(file_of(alg))
}

/// Every `kid` a token in the corpus can carry, with the JWK it names. The suite's
/// own tokens name a key by the thumbprint in its JWK's `kid`, and this crate's by
/// the URL of its verification method, so each key is listed under both.
pub(super) fn published() -> Map<String, Value> {
    let mut map = Map::new();
    for (_, file) in ALGORITHMS {
        let key = key_file(file);
        let jwk = key["publicKeyJwk"].clone();
        map.insert(key_id(file), jwk.clone());
        let thumbprint = jwk["kid"].as_str().expect("a kid").to_owned();
        map.insert(thumbprint, jwk);
    }
    map
}
