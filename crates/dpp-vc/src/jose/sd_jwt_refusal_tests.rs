//! Signing as an SD-JWT: the claims that must stay in the clear, what SD-JWT reserves, and
//! what the `vc+jwt` form refuses, which is refused here.

use base64::Engine as _;
use serde_json::{Value, json};

use dpp_crypto::jws::KeyAlgorithm;
use dpp_crypto::sd_jwt::{ClaimPath, SdJwt};

use super::{
    JoseError, sign_credential_sd_jwt, sign_presentation_sd_jwt, verify_presentation_sd_jwt,
};
use crate::test_support::{KID, credential, generated, presentation};

fn segment(token: &str, index: usize) -> Value {
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(token.split('.').nth(index).expect("a segment"))
        .expect("base64url");
    serde_json::from_slice(&bytes).expect("JSON")
}

/// A change to a document, and what the refusal of the changed document must be.
type Change = Box<dyn Fn(&mut Value)>;
type Refusal = fn(&JoseError) -> bool;

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
    )
}

// ── claims that must stay in the clear ────────────────────────────────────────

#[test]
fn a_claim_that_must_stay_in_the_clear_is_refused_by_name() {
    let mut document = credential();
    document["credentialStatus"] =
        json!({"id": "https://issuer.example/status/1#1", "type": "BitstringStatusListEntry"});
    document["credentialSchema"] =
        json!({"id": "https://issuer.example/schema.json", "type": "JsonSchema"});
    document["relatedResource"] =
        json!([{"id": "https://issuer.example/r", "digestSRI": "sha256-x"}]);
    document["iss"] = json!("https://issuer.example");
    document["aud"] = json!("https://verifier.example");
    document["exp"] = json!(4_000_000_000_u64);
    document["nbf"] = json!(1_000_000_000_u64);
    document["cnf"] = json!({"kid": "k"});

    for name in [
        "@context",
        "type",
        "credentialStatus",
        "credentialSchema",
        "relatedResource",
        "iss",
        "aud",
        "exp",
        "nbf",
        "cnf",
    ] {
        let refused = sign_credential_sd_jwt(
            &document,
            &generated(KeyAlgorithm::Ed25519),
            KID,
            &["cnf"],
            &[path(&json!([name]))],
        );

        assert_eq!(
            refused.map(|_| ()),
            Err(JoseError::UnconcealableClaim {
                name: name.to_owned()
            }),
            "{name}"
        );
    }
}

/// The refusal is for the claim and for any part of it: a path into `credentialSchema`
/// hides part of what a verifier needs, and a path to an element of `type` hides a term.
#[test]
fn a_path_into_a_claim_that_must_stay_in_the_clear_is_refused_too() {
    let mut document = credential();
    document["credentialSchema"] =
        json!({"id": "https://issuer.example/schema.json", "type": "JsonSchema"});

    for segments in [json!(["credentialSchema", "id"]), json!(["type", 0])] {
        let refused = sign(&document, &[path(&segments)]);

        assert!(
            matches!(refused, Err(JoseError::UnconcealableClaim { .. })),
            "{segments}"
        );
    }
}

/// The rule is about a claim at the root. A member of the subject that happens to be named
/// `type` is the subject's own.
#[test]
fn a_member_called_type_deeper_in_the_document_may_be_concealed() {
    let mut document = credential();
    document["credentialSubject"]["type"] = json!("Person");

    let sd_jwt = sign(&document, &[path(&json!(["credentialSubject", "type"]))]).expect("signs");

    assert_eq!(sd_jwt.disclosures().len(), 1);
}

#[test]
fn a_presentation_may_conceal_its_holder_and_one_envelope_and_not_its_type() {
    let key = generated(KeyAlgorithm::EcdsaP384);
    let document = presentation(vec![json!({
        "@context": "https://www.w3.org/ns/credentials/v2",
        "type": "EnvelopedVerifiableCredential",
        "id": "data:application/vc+sd-jwt,opaque",
    })]);

    let sd_jwt = sign_presentation_sd_jwt(
        &document,
        &key,
        KID,
        &[],
        &[
            path(&json!(["holder"])),
            path(&json!(["verifiableCredential", 0])),
        ],
    )
    .expect("signs");

    assert_eq!(sd_jwt.disclosures().len(), 2);
    let payload = segment(sd_jwt.jwt(), 1);
    assert!(payload.get("holder").is_none());
    assert!(payload["verifiableCredential"][0]["..."].is_string());
    for name in ["type", "@context"] {
        assert_eq!(
            sign_presentation_sd_jwt(&document, &key, KID, &[], &[path(&json!([name]))])
                .map(|_| ()),
            Err(JoseError::UnconcealableClaim {
                name: name.to_owned()
            })
        );
    }
}

// ── what SD-JWT reserves ──────────────────────────────────────────────────────

#[test]
fn a_member_the_mechanism_reserves_is_refused_at_any_depth() {
    for (name, plant) in [
        ("_sd", json!(["x"])),
        ("_sd_alg", json!("sha-256")),
        ("...", json!("digest")),
    ] {
        let mut document = credential();
        document["credentialSubject"][name] = plant;

        let refused = sign(&document, &[]);

        assert!(
            matches!(refused, Err(JoseError::ReservedSdJwtMember { name: found }) if found == name),
            "{name}"
        );
    }
    let mut in_array = credential();
    in_array["credentialSubject"]["list"] = json!([{"_sd": []}]);
    assert!(matches!(
        sign(&in_array, &[]),
        Err(JoseError::ReservedSdJwtMember { name: "_sd" })
    ));
}

// ── what the vc+jwt form refuses, this refuses ────────────────────────────────

#[test]
fn what_sign_credential_refuses_is_refused_here() {
    let name = [path(&json!(["credentialSubject", "name"]))];
    let cases: [(Change, Refusal); 7] = [
        (Box::new(|d| d["undeclared"] = json!(1)), |e| {
            matches!(e, JoseError::UndeclaredMembers { .. })
        }),
        (Box::new(|d| d["vc"] = json!({})), |e| {
            matches!(e, JoseError::ReservedClaim { name: "vc" })
        }),
        (
            Box::new(|d| d["@context"] = json!(["https://example.org/other"])),
            |e| matches!(e, JoseError::InvalidContext { .. }),
        ),
        (
            Box::new(|d| d["iss"] = json!("https://elsewhere.example")),
            |e| matches!(e, JoseError::IssuerMismatch { .. }),
        ),
        (Box::new(|d| d["exp"] = json!("tomorrow")), |e| {
            matches!(e, JoseError::MalformedTemporalClaim { claim: "exp" })
        }),
        (Box::new(|d| d["type"] = json!(["Other"])), |e| {
            matches!(e, JoseError::InvalidType { .. })
        }),
        (Box::new(|d| d["credentialSubject"] = json!([])), |e| {
            matches!(e, JoseError::InvalidCredentialSubject { .. })
        }),
    ];
    for (index, (change, expected)) in cases.iter().enumerate() {
        let mut document = credential();
        change(&mut document);

        let refused = sign(&document, &name).expect_err("refused");

        assert!(expected(&refused), "case {index}: {refused:?}");
    }
}

#[test]
fn a_kid_that_is_not_an_absolute_url_is_refused_without_an_iss() {
    let key = generated(KeyAlgorithm::Ed25519);

    let credential = sign_credential_sd_jwt(&credential(), &key, "thumbprint", &[], &[]);
    let presentation =
        sign_presentation_sd_jwt(&presentation(vec![]), &key, "thumbprint", &[], &[]);

    let expected = Err(JoseError::KidNotAbsolute {
        kid: Some("thumbprint".to_owned()),
    });
    assert_eq!(credential.map(|_| ()), expected);
    assert_eq!(presentation.map(|_| ()), expected);
}

#[test]
fn something_that_is_not_an_object_is_refused() {
    let key = generated(KeyAlgorithm::Ed25519);

    assert_eq!(
        sign_credential_sd_jwt(&json!([]), &key, KID, &[], &[]).map(|_| ()),
        Err(JoseError::NotAnObject { what: "credential" })
    );
    assert_eq!(
        sign_presentation_sd_jwt(&json!("p"), &key, KID, &[], &[]).map(|_| ()),
        Err(JoseError::NotAnObject {
            what: "presentation"
        })
    );
}

#[test]
fn a_presentation_signed_as_an_sd_jwt_verifies_back_with_its_holder_concealed() {
    let key = generated(KeyAlgorithm::EcdsaP256);
    let document = presentation(vec![]);

    let sd_jwt = sign_presentation_sd_jwt(&document, &key, KID, &[], &[path(&json!(["holder"]))])
        .expect("signs");

    let verified =
        verify_presentation_sd_jwt(&sd_jwt.serialise(), &key.public_key(), None).expect("verifies");
    assert_eq!(verified.payload(), &document);
}
