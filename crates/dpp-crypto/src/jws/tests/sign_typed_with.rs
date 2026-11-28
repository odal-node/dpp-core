//! [`signer::sign_typed_with`] returns the token [`signer::sign_typed`] returns, for
//! every algorithm the crate signs under.
//!
//! The two take the same key by different routes, a store and the entry itself, so
//! the one property that matters is that the route changes nothing: not the header,
//! not the payload bytes, not the signature. Signing is deterministic under every
//! algorithm here (RFC 8032 for Ed25519, RFC 6979 nonces for ECDSA), which is what
//! lets two separate signings be compared byte for byte.

use serde_json::json;

use super::super::algorithm::KeyAlgorithm;
use super::super::signer;
use super::super::verifier::verify_jws;
use crate::test_support::open_empty_store;
use crate::test_support::temp_path;

const ALGORITHMS: [KeyAlgorithm; 4] = [
    KeyAlgorithm::Ed25519,
    KeyAlgorithm::EcdsaP256,
    KeyAlgorithm::EcdsaP384,
    KeyAlgorithm::EcdsaP521,
];

#[test]
fn both_routes_give_the_same_bytes_for_every_algorithm() {
    let payload = json!({"b": 2, "a": [1, {"z": null}], "m": "text"});
    for algorithm in ALGORITHMS {
        let store = open_empty_store(&temp_path("sign-typed-with"));
        store
            .generate_key_with("k", algorithm)
            .expect("generate key");
        let entry = store.load_key("k").expect("load key");

        for (typ, cty) in [
            (None, None),
            (Some("vc+jwt"), Some("vc")),
            (Some("vp+jwt"), None),
            (None, Some("vp")),
        ] {
            let via_store = signer::sign_typed(&store, "k", &payload, "kid-1", typ, cty)
                .expect("sign through the store");
            let with_key = signer::sign_typed_with(&entry, &payload, "kid-1", typ, cty)
                .expect("sign with the key");
            assert_eq!(via_store, with_key, "{algorithm:?} typ {typ:?} cty {cty:?}");
        }
    }
}

#[test]
fn the_token_from_the_key_verifies_under_the_public_key() {
    let payload = json!({"claim": "value"});
    for algorithm in ALGORITHMS {
        let store = open_empty_store(&temp_path("sign-typed-with-verify"));
        let entry = store
            .generate_key_with("k", algorithm)
            .expect("generate key");

        let jws = signer::sign_typed_with(&entry, &payload, "kid-1", Some("vc+jwt"), Some("vc"))
            .expect("sign with the key");

        verify_jws(&jws, &entry.public_key()).expect("well-formed");
    }
}
