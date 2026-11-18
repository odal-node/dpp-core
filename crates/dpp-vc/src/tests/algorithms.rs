//! The `did:web` document for every algorithm a key can be generated under, and for
//! a document that holds keys of two algorithms after a rotation.

use base64::Engine;
use dpp_crypto::jws::{KeyAlgorithm, PublicKey};

use crate::did_builder::build_did_document;
use crate::test_support::{temp_store, temp_store_with};

const BASE_URL: &str = "https://id.example.com";

/// Each algorithm with the JWK it is published under: `kty`, `crv`, `alg`, and the
/// octets of one coordinate (RFC 8037 section 2 and RFC 7518 section 6.2.1).
const SHAPES: [(KeyAlgorithm, &str, &str, &str, usize); 4] = [
    (KeyAlgorithm::Ed25519, "OKP", "Ed25519", "EdDSA", 32),
    (KeyAlgorithm::EcdsaP256, "EC", "P-256", "ES256", 32),
    (KeyAlgorithm::EcdsaP384, "EC", "P-384", "ES384", 48),
    (KeyAlgorithm::EcdsaP521, "EC", "P-521", "ES512", 66),
];

/// Each algorithm's key is listed in its own JWK shape, and that shape reads back as
/// the same key under the same algorithm, so a verifier never has to guess one.
#[test]
fn each_algorithm_is_published_in_its_own_jwk_shape() {
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    for (algorithm, kty, crv, alg, coordinate) in SHAPES {
        let store = temp_store_with("did-shape", "issuer", algorithm);
        let doc = build_did_document(&store, BASE_URL, "issuer").expect("build");
        let jwk = &doc["verificationMethod"][0]["publicKeyJwk"];

        assert_eq!(jwk["kty"], kty, "{alg}: kty");
        assert_eq!(jwk["crv"], crv, "{alg}: crv");
        assert_eq!(jwk["alg"], alg, "{alg}: alg");
        assert!(
            jwk.get("d").is_none(),
            "{alg}: a private member in a published JWK"
        );

        let x = b64
            .decode(jwk["x"].as_str().expect("x is a string"))
            .expect("x is base64url");
        assert_eq!(x.len(), coordinate, "{alg}: x is one full coordinate");
        if kty == "EC" {
            let y = b64
                .decode(jwk["y"].as_str().expect("y is a string"))
                .expect("y is base64url");
            assert_eq!(y.len(), coordinate, "{alg}: y is one full coordinate");
        } else {
            assert!(jwk.get("y").is_none(), "{alg}: an OKP key has no y");
        }

        assert_eq!(
            PublicKey::from_jwk(jwk).map(|key| key.algorithm()),
            Some(algorithm),
            "{alg}: the published JWK reads back as a key of this algorithm"
        );
    }
}

/// After a rotation across algorithms the document lists the current key and the
/// archived one, each in its own shape and under its own identifier.
#[test]
fn a_rotated_document_lists_each_key_in_its_own_shape() {
    let store = temp_store("did-mixed", "issuer");
    store
        .rotate_key_to("issuer", KeyAlgorithm::EcdsaP256)
        .expect("rotate to P-256");
    let doc = build_did_document(&store, BASE_URL, "issuer").expect("build");

    let methods = doc["verificationMethod"].as_array().expect("an array");
    let shapes: Vec<(&str, &str)> = methods
        .iter()
        .map(|vm| {
            (
                vm["publicKeyJwk"]["kty"].as_str().expect("kty"),
                vm["publicKeyJwk"]["alg"].as_str().expect("alg"),
            )
        })
        .collect();
    assert_eq!(
        shapes,
        [("EC", "ES256"), ("OKP", "EdDSA")],
        "the current P-256 key first, then the archived Ed25519 key"
    );
    assert_ne!(
        methods[0]["id"], methods[1]["id"],
        "each key keeps its own identifier"
    );
}
