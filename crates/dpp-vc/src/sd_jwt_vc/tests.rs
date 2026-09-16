//! SD-JWT VC issuance, presentation and verification.
//!
//! The properties asserted here are the merge gate for this feature, restated
//! against RFC 9901 and draft-ietf-oauth-sd-jwt-vc-19 as read on 2026-09-16.
//! Where the two differ from what was originally sketched, the specification
//! wins and the deviation is named in the test that proves it.

use base64::Engine;
use serde_json::{Value, json};

use dpp_crypto::keystore::KeyStore;
use dpp_domain::ProductGroup;
use dpp_domain::access::{DocumentScope, ProductGroupAccessPolicy};

use super::{SdJwtVcError, TYP, build_issuer_metadata, issue, vct_for, verify};
use crate::test_support::temp_store;

const KEY_ID: &str = "sd-jwt-issuer";
const ISSUER: &str = "https://passports.operator.example";
const SCHEMA_VERSION: &str = "2.6.0";

fn policy() -> ProductGroupAccessPolicy {
    ProductGroupAccessPolicy::for_passport("battery", SCHEMA_VERSION)
        .expect("battery 2.6.0 schema is embedded")
}

/// A passport-shaped payload: an envelope with a product-group payload beneath
/// it, carrying fields from each of the four disclosure classes.
fn payload() -> Value {
    json!({
        "id": "018f3a4c-0000-7000-8000-000000000001",
        "productGroup": "battery",
        "schemaVersion": SCHEMA_VERSION,
        "batchId": "BATCH-7781",
        "productGroupData": {
            "batteryChemistry": "LFP",
            "cathodeMaterial": ["LiFePO4"],
            "stateOfHealthPct": 87.5,
            "testReportResults": "report-4471",
            "safetyMeasures": ["Do not puncture"],
        }
    })
}

fn public_key_b64(store: &KeyStore) -> String {
    let info = store.public_key(KEY_ID).expect("key present");
    // `verify_jws` decodes with URL_SAFE_NO_PAD, so the caller must encode the
    // same way — a STANDARD-encoded key silently fails to verify.
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(hex::decode(&info.verifying_key_hex).unwrap())
}

fn issued(store: &KeyStore) -> dpp_crypto::sd_jwt::SdJwt {
    issue(
        store,
        KEY_ID,
        &payload(),
        &policy(),
        ISSUER,
        &vct_for(ProductGroup::Battery, SCHEMA_VERSION),
        1_789_000_000,
    )
    .expect("issue")
}

/// The four classes are not a guess — they come from the embedded schema, and
/// this asserts the fixture actually spans them. Without it, a later schema
/// change could reclassify a field and quietly turn the tests below into a
/// weaker test that still passes.
#[test]
fn the_fixture_spans_a_public_and_three_non_public_classes() {
    use dpp_domain::Disclosure;
    let policy = policy();
    let class = |k: &str| policy.disclosure_for_path(&[k], DocumentScope::ProductGroupData);

    assert_eq!(class("batteryChemistry"), Disclosure::Public);
    assert_eq!(class("cathodeMaterial"), Disclosure::Restricted);
    assert_eq!(class("stateOfHealthPct"), Disclosure::Individual);
    assert_eq!(class("testReportResults"), Disclosure::Conformity);
}

#[test]
fn a_passport_issues_as_an_sd_jwt_vc_and_the_full_credential_verifies() {
    let store = temp_store("sdjwtvc-issue", KEY_ID);
    let credential = issued(&store).serialise();

    let disclosed = verify(
        &credential,
        &public_key_b64(&store),
        &vct_for(ProductGroup::Battery, SCHEMA_VERSION),
    )
    .expect("full credential verifies");

    assert_eq!(disclosed["iss"], json!(ISSUER));
    assert_eq!(
        disclosed["vct"],
        json!("tag:odal-node.io,2026:vct:battery:2.6.0")
    );
    let pgd = disclosed["productGroupData"].as_object().unwrap();
    assert_eq!(pgd["cathodeMaterial"], json!(["LiFePO4"]));
    assert_eq!(pgd["stateOfHealthPct"], json!(87.5));
    assert_eq!(pgd["testReportResults"], json!("report-4471"));
}

#[test]
fn the_issuer_signed_jwt_declares_dc_plus_sd_jwt() {
    let store = temp_store("sdjwtvc-typ", KEY_ID);
    let jwt = issued(&store).jwt().to_owned();
    let header: Value = serde_json::from_slice(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(jwt.split('.').next().unwrap())
            .unwrap(),
    )
    .unwrap();

    assert_eq!(header["typ"], json!(TYP));
    assert_eq!(header["typ"], json!("dc+sd-jwt"));
    assert_eq!(header["alg"], json!("EdDSA"));
    // Clause 4.2 recommends a `kid` the issuer metadata can be looked up by.
    assert!(header["kid"].is_string());
}

/// Every non-public claim is concealed, at both scopes and at depth.
#[test]
fn no_non_public_value_appears_anywhere_in_the_signed_payload() {
    let store = temp_store("sdjwtvc-conceal", KEY_ID);
    let jwt = issued(&store).jwt().to_owned();
    let payload_json = String::from_utf8(
        base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(jwt.split('.').nth(1).unwrap())
            .unwrap(),
    )
    .unwrap();

    // Envelope scope: `batchId` is restricted.
    assert!(!payload_json.contains("BATCH-7781"));
    assert!(!payload_json.contains("batchId"));
    // Product-group scope, three classes.
    for needle in [
        "LiFePO4",
        "87.5",
        "report-4471",
        "Do not puncture",
        "cathodeMaterial",
        "stateOfHealthPct",
        "testReportResults",
        "safetyMeasures",
    ] {
        assert!(!payload_json.contains(needle), "{needle} is in the payload");
    }

    // The public ones are still readable without any disclosure.
    assert!(payload_json.contains("LFP"));
    assert!(payload_json.contains("batteryChemistry"));
}

/// The merge gate's core property, over the presentation bytes.
#[test]
fn a_two_of_four_presentation_verifies_and_withholds_the_other_two() {
    let store = temp_store("sdjwtvc-present", KEY_ID);
    let credential = issued(&store);
    let vct = vct_for(ProductGroup::Battery, SCHEMA_VERSION);

    let presentation = credential
        .present(&["cathodeMaterial", "stateOfHealthPct"])
        .serialise();

    let disclosed =
        verify(&presentation, &public_key_b64(&store), &vct).expect("presentation verifies");

    let pgd = disclosed["productGroupData"].as_object().unwrap();
    assert_eq!(pgd["cathodeMaterial"], json!(["LiFePO4"]));
    assert_eq!(pgd["stateOfHealthPct"], json!(87.5));
    assert!(!pgd.contains_key("testReportResults"));
    assert!(!pgd.contains_key("safetyMeasures"));

    // Asserted over the bytes, not inferred from the construction — including
    // after base64url-decoding every segment of the presentation.
    let decoded: String = presentation
        .split('~')
        .flat_map(|s| s.split('.'))
        .filter_map(|s| {
            base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(s)
                .ok()
        })
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .collect::<Vec<_>>()
        .join("|");
    for needle in ["report-4471", "Do not puncture", "BATCH-7781"] {
        assert!(!presentation.contains(needle), "{needle} on the wire");
        assert!(!decoded.contains(needle), "{needle} decodes out");
    }

    // The signature covers the same bytes it covered at issuance — the holder
    // never re-signs, and could not.
    assert_eq!(
        presentation.split('~').next(),
        credential.serialise().split('~').next()
    );
}

/// Withdrawal: the property the frozen signed views cannot give.
///
/// A claim reclassified after issuance is withheld by presenting without its
/// disclosure. No re-signing, and the credential still verifies — which is the
/// whole argument for this feature over re-signing per audience.
#[test]
fn withholding_a_claim_needs_no_new_signature() {
    let store = temp_store("sdjwtvc-withdraw", KEY_ID);
    let credential = issued(&store);
    let vct = vct_for(ProductGroup::Battery, SCHEMA_VERSION);

    let everything = verify(&credential.serialise(), &public_key_b64(&store), &vct).unwrap();
    assert!(everything["productGroupData"]["stateOfHealthPct"].is_number());

    let withheld = credential.present(&["cathodeMaterial"]).serialise();
    let after = verify(&withheld, &public_key_b64(&store), &vct).expect("still verifies");
    assert!(
        !after["productGroupData"]
            .as_object()
            .unwrap()
            .contains_key("stateOfHealthPct")
    );
}

/// Tampering, corrected against what the mechanism actually does: an unmatched
/// disclosure makes the **whole credential** unreadable, rather than the claim
/// quietly disappearing.
#[test]
fn tampering_with_a_revealed_value_refuses_the_credential() {
    use dpp_crypto::sd_jwt::{Disclosure, SdJwt, SdJwtError};

    let store = temp_store("sdjwtvc-tamper", KEY_ID);
    let credential = issued(&store);
    let original = credential
        .disclosures()
        .iter()
        .find(|d| d.claim_name() == "stateOfHealthPct")
        .expect("state of health is disclosable");

    let forged = Disclosure::with_salt(
        original.salt().to_owned(),
        original.claim_name(),
        json!(12.0),
    );
    let tampered = SdJwt::new(credential.jwt().to_owned(), vec![forged]).serialise();

    assert_eq!(
        verify(
            &tampered,
            &public_key_b64(&store),
            &vct_for(ProductGroup::Battery, SCHEMA_VERSION)
        ),
        Err(SdJwtVcError::SdJwt(SdJwtError::UnusedDisclosures(1)))
    );
}

#[test]
fn a_credential_signed_by_another_key_does_not_verify() {
    let store = temp_store("sdjwtvc-good", KEY_ID);
    let other = temp_store("sdjwtvc-other", KEY_ID);
    let credential = issued(&store).serialise();

    assert_eq!(
        verify(
            &credential,
            &public_key_b64(&other),
            &vct_for(ProductGroup::Battery, SCHEMA_VERSION)
        ),
        Err(SdJwtVcError::BadSignature)
    );
}

/// The signature is checked before any claim is read, so a forged token never
/// reaches the claim logic.
#[test]
fn a_mutated_signature_is_refused_before_the_claims_are_read() {
    let store = temp_store("sdjwtvc-mutate", KEY_ID);
    let credential = issued(&store).serialise();
    let mut bytes: Vec<char> = credential.chars().collect();
    let sig_start = credential.find('~').unwrap();
    let last_sig_char = credential[..sig_start].rfind(|c: char| c != '.').unwrap();
    bytes[last_sig_char] = if bytes[last_sig_char] == 'A' {
        'B'
    } else {
        'A'
    };
    let mutated: String = bytes.into_iter().collect();

    assert_eq!(
        verify(
            &mutated,
            &public_key_b64(&store),
            &vct_for(ProductGroup::Battery, SCHEMA_VERSION)
        ),
        Err(SdJwtVcError::BadSignature)
    );
}

#[test]
fn a_credential_of_another_type_is_refused() {
    let store = temp_store("sdjwtvc-vct", KEY_ID);
    let credential = issued(&store).serialise();

    let err = verify(
        &credential,
        &public_key_b64(&store),
        &vct_for(ProductGroup::Battery, "2.5.0"),
    )
    .unwrap_err();

    assert!(matches!(err, SdJwtVcError::UnexpectedVct { .. }));
}

/// Two issuances of the same passport must not be linkable by their digests.
#[test]
fn two_issuances_of_one_passport_share_no_digest() {
    let store = temp_store("sdjwtvc-relink", KEY_ID);
    let first = issued(&store);
    let second = issued(&store);

    let digests = |c: &dpp_crypto::sd_jwt::SdJwt| -> Vec<String> {
        c.disclosures().iter().map(|d| d.digest()).collect()
    };
    let a = digests(&first);
    let b = digests(&second);
    assert!(
        a.iter().all(|d| !b.contains(d)),
        "a digest repeated across issuances"
    );
    // And the signed tokens therefore differ too.
    assert_ne!(first.jwt(), second.jwt());
}

// ── JWT VC Issuer Metadata ───────────────────────────────────────────────────

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
