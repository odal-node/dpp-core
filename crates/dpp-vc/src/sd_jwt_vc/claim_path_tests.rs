//! Choosing what a credential presents by claim path: a path into the passport
//! starts at [`PASSPORT_CLAIM`].

use serde_json::{Map, Value, json};

use dpp_crypto::keystore::KeyStore;
use dpp_crypto::sd_jwt::{ClaimPath, SdJwt, SdJwtError};
use dpp_domain::ProductGroup;

use super::test_fixtures::*;
use super::{PASSPORT_CLAIM, vct_for, verify};
use crate::test_support::temp_store;

/// A restricted member of the fixture passport, as a path into the credential.
fn cathode_material() -> ClaimPath {
    ClaimPath::claim(PASSPORT_CLAIM)
        .key("productGroupData")
        .key("cathodeMaterial")
}

/// What a Verifier reads from `credential` presented with the Disclosures `chosen`.
fn presented(store: &KeyStore, credential: &SdJwt, chosen: &[String]) -> Map<String, Value> {
    let chosen: Vec<&str> = chosen.iter().map(String::as_str).collect();
    let presentation = credential.present(&chosen).expect("present").serialise();
    verify(
        &presentation,
        &public_key_b64(store),
        &vct_for(ProductGroup::Battery, SCHEMA_VERSION),
        now(),
        None,
    )
    .expect("the presentation verifies")
}

#[test]
fn a_path_into_the_passport_starts_at_the_passport_claim() {
    let store = temp_store("sd-jwt-vc-paths", KEY_ID);
    let credential = issued(&store);

    let chosen = credential
        .digests_for_paths(&[cathode_material()])
        .expect("the credential holds the claim");

    // Exactly what a presentation of nothing shows, and the one claim chosen.
    let mut expected = presented(&store, &credential, &[]);
    expected[PASSPORT_CLAIM]["productGroupData"]["cathodeMaterial"] = json!(["LiFePO4"]);
    assert_eq!(presented(&store, &credential, &chosen), expected);
}

/// The path a caller would write from the passport alone names nothing in the
/// credential, and is refused rather than presenting less than was meant.
#[test]
fn a_path_without_the_passport_segment_is_refused() {
    let store = temp_store("sd-jwt-vc-paths", KEY_ID);
    let credential = issued(&store);
    let from_the_passport = ClaimPath::claim("productGroupData").key("cathodeMaterial");

    assert_eq!(
        credential.digests_for_paths(&[from_the_passport]),
        Err(SdJwtError::ClaimNotInToken(
            r#"["productGroupData","cathodeMaterial"]"#.to_owned()
        ))
    );
}
