//! JWT VC Issuer Metadata — the document a verifier fetches to get the key.

use base64::Engine;
use serde_json::{Value, json};

use super::build_issuer_metadata;
use super::test_fixtures::*;
use crate::test_support::temp_store;

#[test]
fn issuer_metadata_carries_the_issuer_and_exactly_one_key_source() {
    let store = temp_store("sdjwtvc-meta", KEY_ID);
    let metadata = build_issuer_metadata(&store, ISSUER, KEY_ID).expect("metadata");

    assert_eq!(metadata["issuer"], json!(ISSUER));
    // Clause 4.2: either `jwks` or `jwks_uri`, "but not both".
    assert!(metadata.get("jwks").is_some());
    assert!(metadata.get("jwks_uri").is_none());

    let keys = metadata["jwks"]["keys"].as_array().unwrap();
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0]["kty"], json!("OKP"));
    assert_eq!(keys[0]["crv"], json!("Ed25519"));
    assert_eq!(keys[0]["alg"], json!("EdDSA"));
    assert_eq!(keys[0]["use"], json!("sig"));
}

/// The `kid` a verifier looks the key up by must be the one the token names,
/// or the recommendation in clause 4.2 is satisfied in name only.
#[test]
fn the_metadata_kid_matches_the_kid_in_the_signed_token() {
    let store = temp_store("sdjwtvc-kid", KEY_ID);
    let jwt = issued(&store).jwt().to_owned();
    let header: Value = serde_json::from_slice(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(jwt.split('.').next().unwrap())
            .unwrap(),
    )
    .unwrap();

    let metadata = build_issuer_metadata(&store, ISSUER, KEY_ID).unwrap();
    assert_eq!(metadata["jwks"]["keys"][0]["kid"], header["kid"]);
}

#[test]
fn issuer_metadata_is_none_for_a_key_the_store_does_not_have() {
    let store = temp_store("sdjwtvc-nokey", KEY_ID);
    assert!(build_issuer_metadata(&store, ISSUER, "absent").is_none());
}

/// A private key must never reach the published document.
#[test]
fn issuer_metadata_carries_no_private_key_material() {
    let store = temp_store("sdjwtvc-private", KEY_ID);
    let metadata = build_issuer_metadata(&store, ISSUER, KEY_ID).unwrap();
    let serialised = metadata.to_string();
    for private_member in ["\"d\"", "\"p\"", "\"q\"", "privateKey"] {
        assert!(
            !serialised.contains(private_member),
            "{private_member} in published metadata"
        );
    }
}
