//! Signing as an SD-JWT: what goes into the header and payload, which claims a path may
//! conceal, and what is refused before anything is signed.

use base64::Engine as _;
use serde_json::{Value, json};

use dpp_crypto::jws::KeyAlgorithm;
use dpp_crypto::keystore::KeyEntry;
use dpp_crypto::sd_jwt::{ClaimPath, SdJwt, holder_key_confirmation};

use super::VerifyOptions;
use super::{
    JoseError, sign_credential_sd_jwt, sign_presentation_sd_jwt, verify_credential_sd_jwt,
};
use crate::test_support::NOT_REQUIRED;
use crate::test_support::{EVERY_ALGORITHM, KID, credential, generated, presentation};

fn segment(token: &str, index: usize) -> Value {
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(token.split('.').nth(index).expect("a segment"))
        .expect("base64url");
    serde_json::from_slice(&bytes).expect("JSON")
}

/// A credential whose subject has a nested object and an array, to conceal parts of.
fn rich() -> Value {
    let mut document = credential();
    document["credentialSubject"] = json!({
        "id": "did:example:subject",
        "name": "Example",
        "address": {"street": "1 Main St", "city": "Anytown"},
        "phones": ["+1-555-0001", "+1-555-0002", "+1-555-0003"],
    });
    document
}

fn path(segments: &Value) -> ClaimPath {
    ClaimPath::from_json(segments).expect("a path")
}

fn sign(document: &Value, disclose: &[ClaimPath]) -> Result<SdJwt, JoseError> {
    sign_credential_sd_jwt(
        document,
        &generated(KeyAlgorithm::EcdsaP256),
        KID,
        &[],
        disclose,
        None,
    )
}

#[test]
fn the_header_names_the_key_the_kid_and_the_type() {
    let key = generated(KeyAlgorithm::EcdsaP384);

    let sd_jwt = sign_credential_sd_jwt(&credential(), &key, KID, &[], &[], None).expect("signs");

    assert_eq!(
        segment(sd_jwt.jwt(), 0),
        json!({"alg": "ES384", "kid": KID, "typ": "vc+sd-jwt", "cty": "vc"})
    );
    let vp =
        sign_presentation_sd_jwt(&presentation(vec![]), &key, KID, &[], &[], None).expect("signs");
    assert_eq!(
        segment(vp.jwt(), 0),
        json!({"alg": "ES384", "kid": KID, "typ": "vp+sd-jwt", "cty": "vp"})
    );
}

/// With nothing concealed the payload is the document, with no `_sd_alg` and no `_sd`, and
/// the token is the Issuer-signed JWT and its closing `~`.
#[test]
fn with_no_path_the_payload_is_the_document_and_nothing_is_concealed() {
    let document = credential();

    let sd_jwt = sign(&document, &[]).expect("signs");

    assert_eq!(segment(sd_jwt.jwt(), 1), document);
    assert!(sd_jwt.disclosures().is_empty());
    assert_eq!(sd_jwt.serialise(), format!("{}~", sd_jwt.jwt()));
}

#[test]
fn a_concealed_claim_is_a_digest_in_the_token_and_the_whole_document_is_what_reads_back() {
    let document = credential();
    let key = generated(KeyAlgorithm::Ed25519);

    let sd_jwt = sign_credential_sd_jwt(
        &document,
        &key,
        KID,
        &[],
        &[path(&json!(["credentialSubject", "name"]))],
        None,
    )
    .expect("signs");

    let payload = segment(sd_jwt.jwt(), 1);
    assert_eq!(payload["_sd_alg"], "sha-256");
    assert!(payload["credentialSubject"].get("name").is_none());
    // One digest for the claim, padded with decoys to a multiple of eight.
    assert_eq!(
        payload["credentialSubject"]["_sd"].as_array().map(Vec::len),
        Some(8)
    );
    assert!(payload.get("_sd").is_none());
    assert!(!payload.to_string().contains("Example"));
    assert_eq!(sd_jwt.disclosures().len(), 1);
    assert_eq!(sd_jwt.disclosures()[0].claim_name(), Some("name"));
    let verified = verify_credential_sd_jwt(
        &sd_jwt.serialise(),
        &key.public_key(),
        VerifyOptions::new(),
        &NOT_REQUIRED,
    );
    assert_eq!(verified.expect("verifies"), document);
}

#[test]
fn every_algorithm_signs_and_verifies_back_to_the_document() {
    for algorithm in EVERY_ALGORITHM {
        let key = generated(algorithm);
        let document = rich();

        let sd_jwt = sign_credential_sd_jwt(
            &document,
            &key,
            KID,
            &[],
            &[path(&json!(["credentialSubject", "address", "street"]))],
            None,
        )
        .expect("signs");

        let verified = verify_credential_sd_jwt(
            &sd_jwt.serialise(),
            &key.public_key(),
            VerifyOptions::new(),
            &NOT_REQUIRED,
        );
        assert_eq!(verified.expect("verifies"), document, "{algorithm:?}");
    }
}

#[test]
fn a_nested_member_is_concealed_in_the_object_it_sits_in() {
    let document = rich();

    let sd_jwt = sign(
        &document,
        &[
            path(&json!(["credentialSubject", "address", "street"])),
            path(&json!(["credentialSubject", "address", "city"])),
        ],
    )
    .expect("signs");

    let payload = segment(sd_jwt.jwt(), 1);
    let address = &payload["credentialSubject"]["address"];
    assert_eq!(address.as_object().map(|a| a.len()), Some(1));
    assert_eq!(address["_sd"].as_array().map(Vec::len), Some(8));
    assert!(payload["credentialSubject"].get("_sd").is_none());
    assert_eq!(sd_jwt.disclosures().len(), 2);
}

/// An element is replaced by `{"...": digest}` (RFC 9901 clause 4.2.2), among decoy
/// placeholders that pad the array to eight (clause 4.2.5). The elements left in the clear
/// keep their order, and the one placeholder that stands for something is the Disclosure's.
#[test]
fn an_array_element_is_a_placeholder_among_decoys_and_the_others_keep_their_order() {
    let document = rich();
    let sd_jwt = sign(
        &document,
        &[path(&json!(["credentialSubject", "phones", 1]))],
    )
    .expect("signs");

    let phones = segment(sd_jwt.jwt(), 1)["credentialSubject"]["phones"]
        .as_array()
        .expect("an array")
        .clone();
    let clear: Vec<&Value> = phones.iter().filter(|item| item.is_string()).collect();
    assert_eq!(clear, vec![&json!("+1-555-0001"), &json!("+1-555-0003")]);
    let placeholders: Vec<&Value> = phones.iter().filter(|item| item.is_object()).collect();
    assert_eq!(
        placeholders.len(),
        8,
        "one for the element and seven decoys"
    );
    assert!(
        placeholders
            .iter()
            .all(|p| p.as_object().map(|p| p.len()) == Some(1) && p["..."].is_string())
    );
    let element = &sd_jwt.disclosures()[0];
    assert!(element.is_array_element());
    assert!(placeholders.contains(&&json!({ "...": element.digest() })));
    assert_eq!(
        Value::Object(sd_jwt.disclosed_payload().expect("reads back")),
        document
    );
}

#[test]
fn a_null_step_conceals_every_element() {
    let document = rich();

    let sd_jwt = sign(
        &document,
        &[path(&json!(["credentialSubject", "phones", null]))],
    )
    .expect("signs");

    assert_eq!(sd_jwt.disclosures().len(), 3);
    let payload = Value::Object(sd_jwt.disclosed_payload().expect("reads back"));
    assert_eq!(payload, document);
}

/// A path to a claim, and a path to a claim inside it: the inner Disclosure travels inside
/// the value of the outer one (RFC 9901 clause 4.2.6), and both read back.
#[test]
fn a_claim_concealed_inside_a_concealed_claim_is_a_disclosure_in_a_disclosure() {
    let document = rich();
    let key = generated(KeyAlgorithm::Ed25519);

    let sd_jwt = sign_credential_sd_jwt(
        &document,
        &key,
        KID,
        &[],
        &[
            path(&json!(["credentialSubject", "address"])),
            path(&json!(["credentialSubject", "address", "street"])),
        ],
        None,
    )
    .expect("signs");

    assert_eq!(sd_jwt.disclosures().len(), 2);
    let outer = sd_jwt
        .disclosures()
        .iter()
        .find(|d| d.claim_name() == Some("address"))
        .expect("the outer Disclosure");
    assert!(outer.claim_value().get("_sd").is_some());
    assert!(outer.claim_value().get("street").is_none());
    let verified = verify_credential_sd_jwt(
        &sd_jwt.serialise(),
        &key.public_key(),
        VerifyOptions::new(),
        &NOT_REQUIRED,
    );
    assert_eq!(verified.expect("verifies"), document);
}

#[test]
fn the_same_path_twice_conceals_once() {
    let name = path(&json!(["credentialSubject", "name"]));

    let sd_jwt = sign(&credential(), &[name.clone(), name]).expect("signs");

    assert_eq!(sd_jwt.disclosures().len(), 1);
}

/// Each Disclosure carries a salt of its own, so two tokens over one document differ.
#[test]
fn signing_twice_gives_two_tokens_that_read_back_alike() {
    let key = generated(KeyAlgorithm::Ed25519);
    let document = credential();
    let name = [path(&json!(["credentialSubject", "name"]))];

    let first = sign_credential_sd_jwt(&document, &key, KID, &[], &name, None).expect("signs");
    let second = sign_credential_sd_jwt(&document, &key, KID, &[], &name, None).expect("signs");

    assert_ne!(first.serialise(), second.serialise());
    assert_eq!(first.disclosed_payload(), second.disclosed_payload());
}

// ── paths that select nothing ─────────────────────────────────────────────────

fn not_found(segments: &Value) -> JoseError {
    JoseError::DisclosurePathNotFound {
        path: path(segments).to_string(),
    }
}

#[test]
fn a_path_that_selects_nothing_is_refused_and_not_ignored() {
    for segments in [
        json!(["credentialSubject", "missing"]),
        json!(["missing"]),
        json!(["credentialSubject", "phones", 3]),
        json!(["credentialSubject", "name", "inner"]),
        json!(["credentialSubject", 0]),
        json!(["credentialSubject", "phones", "first"]),
        json!([0]),
        json!([null]),
        json!(["credentialSubject", "address", null]),
    ] {
        assert_eq!(
            sign(&rich(), &[path(&segments)]).map(|_| ()),
            Err(not_found(&segments)),
            "{segments}"
        );
    }
}

/// A `null` step stands for every element, so every one must hold the rest of the path.
#[test]
fn a_null_step_over_an_element_that_lacks_the_rest_is_refused() {
    let mut document = credential();
    document["credentialSubject"]["degrees"] = json!([{"type": "BA"}, {"name": "no type"}]);
    let segments = json!(["credentialSubject", "degrees", null, "type"]);

    assert_eq!(
        sign(&document, &[path(&segments)]).map(|_| ()),
        Err(not_found(&segments))
    );
}

#[test]
fn a_path_that_selects_nothing_refuses_the_whole_signing_whatever_else_it_holds() {
    let good = path(&json!(["credentialSubject", "name"]));
    let bad = path(&json!(["credentialSubject", "nowhere"]));

    assert!(sign(&credential(), &[good, bad]).is_err());
}

// ── signing names the Holder's key ────────────────────────────────────────────

#[test]
fn a_holder_key_given_at_signing_is_written_as_cnf_jwk() {
    let (issuer, holder) = holder_keys();

    let sd_jwt = issued_to(&issuer, &holder);

    let payload = segment(sd_jwt.jwt(), 1);
    assert_eq!(
        payload["cnf"],
        holder_key_confirmation(&holder.public_key())
    );
    assert_eq!(payload["cnf"]["jwk"], holder.public_key().jwk());
}

#[test]
fn a_presentation_is_given_the_holder_key_the_same_way() {
    let (issuer, holder) = holder_keys();

    let sd_jwt = sign_presentation_sd_jwt(
        &presentation(vec![]),
        &issuer,
        KID,
        &[],
        &[],
        Some(&holder.public_key()),
    )
    .expect("signs");

    assert_eq!(
        segment(sd_jwt.jwt(), 1)["cnf"],
        holder_key_confirmation(&holder.public_key())
    );
}

#[test]
fn no_holder_key_means_no_cnf() {
    let (issuer, _) = holder_keys();

    let sd_jwt =
        sign_credential_sd_jwt(&credential(), &issuer, KID, &[], &[], None).expect("signs");

    assert!(segment(sd_jwt.jwt(), 1).get("cnf").is_none());
}

/// `cnf` is not concealable, with or without a holder key: the refusal of the signer stays.
#[test]
fn cnf_stays_in_the_clear_when_a_holder_key_is_given() {
    let (issuer, holder) = holder_keys();

    let refused = sign_credential_sd_jwt(
        &credential(),
        &issuer,
        KID,
        &[],
        &[ClaimPath::claim("cnf")],
        Some(&holder.public_key()),
    );

    assert_eq!(
        refused.map(|_| ()),
        Err(JoseError::UnconcealableClaim {
            name: "cnf".to_owned()
        })
    );
}

/// A `cnf` the caller wrote is not overwritten by the key it also gave.
#[test]
fn a_holder_key_is_refused_beside_a_cnf_the_document_carries() {
    let (issuer, holder) = holder_keys();
    let mut document = credential();
    document["cnf"] = json!({"kid": "k"});

    let refused = sign_credential_sd_jwt(
        &document,
        &issuer,
        KID,
        &["cnf"],
        &[],
        Some(&holder.public_key()),
    );

    assert_eq!(
        refused.map(|_| ()),
        Err(JoseError::ConfirmationClaimPresent)
    );
}

/// The Issuer's key, and the Holder's, which are not the same key.
fn holder_keys() -> (KeyEntry, KeyEntry) {
    (
        generated(KeyAlgorithm::EcdsaP256),
        generated(KeyAlgorithm::Ed25519),
    )
}

/// A credential issued to `holder`, with `name` concealed.
fn issued_to(issuer: &KeyEntry, holder: &KeyEntry) -> SdJwt {
    sign_credential_sd_jwt(
        &credential(),
        issuer,
        KID,
        &[],
        &[ClaimPath::claim("credentialSubject").key("name")],
        Some(&holder.public_key()),
    )
    .expect("signs")
}
