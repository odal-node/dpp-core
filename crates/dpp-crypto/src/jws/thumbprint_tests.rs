//! The key thumbprint, held to the example RFC 8037 publishes for it.
//!
//! The thumbprint names every key on the wire, so a misreading would give each
//! key an identifier no other implementation derives, and nothing here would
//! notice: the signer and the verifier would agree on the wrong value. RFC 8037
//! appendix A.3 prints the canonical JWK of its example key, the SHA-256 of it,
//! and the thumbprint as RFC 7638 defines it.

use super::algorithm::{JWK_THUMBPRINT_URI_PREFIX, KeyAlgorithm};

/// Appendix A.2's public key, in hexadecimal.
const PUBLIC_KEY_HEX: &str = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";

/// Appendix A.3: the thumbprint, in base64url, and the hash it encodes.
const THUMBPRINT: &str = "kPrK_qmxVWaYVA9wwBF6Iuo3vVzz7TxHCTwXBygrS4k";
const THUMBPRINT_SHA256_HEX: &str =
    "90facafea9b1556698540f70c0117a22ea37bd5cf3ed3c47093c1707282b4b89";

fn public_key() -> Vec<u8> {
    hex::decode(PUBLIC_KEY_HEX).expect("hex")
}

#[test]
fn the_rfc_example_key_has_the_rfc_thumbprint() {
    assert_eq!(
        KeyAlgorithm::Ed25519.jwk_thumbprint(&public_key()),
        THUMBPRINT
    );

    // The same value, decoded: the SHA-256 the RFC prints beside it.
    use base64::Engine;
    let hash = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(THUMBPRINT)
        .expect("base64url");
    assert_eq!(hex::encode(hash), THUMBPRINT_SHA256_HEX);
}

/// The URI form is the prefix that names the hash, then the same thumbprint.
#[test]
fn the_uri_form_prefixes_the_thumbprint_with_its_hash() {
    assert_eq!(
        JWK_THUMBPRINT_URI_PREFIX,
        "urn:ietf:params:oauth:jwk-thumbprint:sha-256:"
    );
    assert_eq!(
        KeyAlgorithm::Ed25519.thumbprint_uri(&public_key()),
        format!("urn:ietf:params:oauth:jwk-thumbprint:sha-256:{THUMBPRINT}")
    );
}

/// The published JWK carries the thumbprint URI as its `kid` and the algorithm
/// as its `alg`, over the same three members as the bare JWK. The thumbprint is
/// of the required members alone, so adding those two changes nothing.
#[test]
fn the_published_jwk_names_itself_without_changing_its_thumbprint() {
    let key = public_key();
    let bare = KeyAlgorithm::Ed25519.public_key_jwk(&key);
    let published = KeyAlgorithm::Ed25519.published_jwk(&key);

    for member in ["kty", "crv", "x"] {
        assert_eq!(published[member], bare[member], "{member}");
    }
    assert_eq!(published["kid"], KeyAlgorithm::Ed25519.thumbprint_uri(&key));
    assert_eq!(published["alg"], "EdDSA");
    assert_eq!(published.as_object().expect("object").len(), 5);
}

#[test]
fn different_keys_have_different_thumbprints() {
    let mut other = public_key();
    other[0] ^= 1;

    assert_ne!(
        KeyAlgorithm::Ed25519.jwk_thumbprint(&public_key()),
        KeyAlgorithm::Ed25519.jwk_thumbprint(&other)
    );
}
