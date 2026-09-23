//! JWT VC Issuer Metadata — the document a verifier fetches to get the key.

use base64::Engine;
use serde_json::{Value, json};

use super::build_issuer_metadata;
use super::issuer_metadata::metadata_for_keys;
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

/// The `kid` of every key the metadata publishes.
fn kids(metadata: &Value) -> Vec<String> {
    metadata["jwks"]["keys"]
        .as_array()
        .expect("a key set")
        .iter()
        .map(|k| {
            k["kid"]
                .as_str()
                .expect("every key names its kid")
                .to_owned()
        })
        .collect()
}

/// A compromise rotation takes the revoked key out of the published set, and
/// serves its replacement alone.
///
/// A verifier trusts whatever this document lists. A revoked key left in it
/// goes on verifying signatures made with it — the failure revocation exists to
/// prevent, reached through the document meant to convey it.
#[test]
fn a_revoked_key_is_not_published_after_a_compromise_rotation() {
    let store = temp_store("sdjwtvc-revoke", KEY_ID);
    let revoked = store
        .load_key(KEY_ID)
        .expect("current key")
        .fingerprint
        .clone();
    let replacement = store
        .revoke_and_rotate(KEY_ID)
        .expect("revoke and rotate")
        .fingerprint
        .clone();

    let metadata = build_issuer_metadata(&store, ISSUER, KEY_ID).expect("metadata");
    assert_eq!(kids(&metadata), [replacement]);
    assert!(!kids(&metadata).contains(&revoked));
}

/// An ordinary rotation keeps the old key, so a credential signed before it
/// still verifies. The filter removes revoked keys, not archived ones.
#[test]
fn a_rotated_key_stays_published() {
    let store = temp_store("sdjwtvc-rotate", KEY_ID);
    let old = store
        .load_key(KEY_ID)
        .expect("current key")
        .fingerprint
        .clone();
    let new = store
        .rotate_key(KEY_ID)
        .expect("rotate")
        .fingerprint
        .clone();

    let metadata = build_issuer_metadata(&store, ISSUER, KEY_ID).expect("metadata");
    let published = kids(&metadata);
    assert_eq!(published.len(), 2, "{published:?}");
    assert!(published.contains(&old) && published.contains(&new));
}

/// The current key is filtered on `revoked` too.
///
/// No store call leaves a current key revoked, so this reaches the branch the
/// only way it can be reached: a real key's public half with the flag set, as a
/// store opened on a record revoked in place would return it.
#[test]
fn a_revoked_current_key_is_not_published() {
    let store = temp_store("sdjwtvc-revoked-current", KEY_ID);
    let mut current = store.public_key(KEY_ID).expect("current key");
    current.revoked = true;

    // Nothing usable is left, so there is no document rather than an empty set.
    assert!(metadata_for_keys(ISSUER, &current, &[]).is_none());

    // An unrevoked archived key is still served, and only that one.
    let archived = store.public_key(KEY_ID).expect("current key");
    let metadata = metadata_for_keys(ISSUER, &current, &[archived]).expect("metadata");
    assert_eq!(kids(&metadata).len(), 1);
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
