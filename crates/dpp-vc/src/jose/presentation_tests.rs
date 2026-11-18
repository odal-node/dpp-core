//! Verifying a presentation: the outer token, the envelope shapes, and what the
//! closure is told.
//!
//! What the caller answers for each envelope is in `decision_tests`.

use base64::Engine as _;
use serde_json::{Value, json};

use dpp_crypto::jws::KeyAlgorithm;
use dpp_crypto::keystore::KeyEntry;

use super::test_fixtures::{
    KID, credential, credential_header, envelope, foreign_envelope, forge, generated, presentation,
    presentation_header,
};
use super::{
    EnvelopeDecision, JoseError, VerifiedPresentation, sign_credential, sign_presentation,
    verify_presentation,
};

fn holder() -> KeyEntry {
    generated(KeyAlgorithm::EcdsaP384)
}

fn skip(_: usize, _: &str, _: Option<&str>) -> EnvelopeDecision {
    EnvelopeDecision::Skip
}

/// A presentation of `members`, signed by `holder`, verified with every envelope
/// skipped; and whether the closure was asked.
fn verify_members(
    holder: &KeyEntry,
    members: Vec<Value>,
) -> (Result<VerifiedPresentation, JoseError>, bool) {
    let token = sign_presentation(&presentation(members), holder, KID, &[]).expect("signs");
    let mut asked = false;
    let result = verify_presentation(&token, &holder.public_key(), None, |_, _, _| {
        asked = true;
        EnvelopeDecision::Skip
    });
    (result, asked)
}

fn issuer_token(issuer: &KeyEntry) -> String {
    sign_credential(&credential(), issuer, KID, &[]).expect("signs")
}

// ── the outer token ───────────────────────────────────────────────────────────

#[test]
fn a_presentation_of_one_verified_credential_round_trips() {
    let holder = holder();
    let issuer = generated(KeyAlgorithm::EcdsaP256);
    let document = presentation(vec![envelope(&issuer_token(&issuer))]);
    let token = sign_presentation(&document, &holder, KID, &[]).expect("signs");

    let verified = verify_presentation(&token, &holder.public_key(), None, |index, media, kid| {
        assert_eq!((index, media, kid), (0, "application/vc+jwt", Some(KID)));
        EnvelopeDecision::Verify(issuer.public_key())
    })
    .expect("verifies");

    assert_eq!(verified.payload, document);
    assert_eq!(verified.verified.len(), 1);
    assert_eq!(verified.verified[0].credential, credential());
    assert!(verified.skipped.is_empty());
}

#[test]
fn a_presentation_with_no_credentials_verifies_with_nothing_to_report() {
    let holder = holder();
    let mut document = presentation(Vec::new());
    document
        .as_object_mut()
        .expect("an object")
        .remove("verifiableCredential");
    let token = sign_presentation(&document, &holder, KID, &[]).expect("signs");

    let verified = verify_presentation(&token, &holder.public_key(), None, |_, _, _| {
        panic!("there is no envelope to ask about")
    })
    .expect("verifies");

    assert!(verified.verified.is_empty() && verified.skipped.is_empty());
}

#[test]
fn a_credential_token_is_not_a_presentation() {
    let holder = holder();
    let token = sign_credential(&credential(), &holder, KID, &[]).expect("signs");

    assert_eq!(
        verify_presentation(&token, &holder.public_key(), None, skip).map(|_| ()),
        Err(JoseError::WrongMediaType {
            member: "typ",
            found: Some("\"vc+jwt\"".to_owned()),
            expected: "vp+jwt",
        })
    );
}

#[test]
fn a_presentation_token_is_not_a_credential() {
    let holder = holder();
    let token = sign_presentation(&presentation(Vec::new()), &holder, KID, &[]).expect("signs");

    assert_eq!(
        super::verify_credential(&token, &holder.public_key(), None),
        Err(JoseError::WrongMediaType {
            member: "typ",
            found: Some("\"vp+jwt\"".to_owned()),
            expected: "vc+jwt",
        })
    );
}

#[test]
fn the_outer_token_is_refused_before_any_envelope_is_read() {
    let holder = holder();
    let other = holder_other();
    let token = forge(
        &other,
        &presentation_header(&other),
        &presentation(vec![json!("not an envelope")]),
    );
    let mut asked = false;

    let refused = verify_presentation(&token, &holder.public_key(), None, |_, _, _| {
        asked = true;
        EnvelopeDecision::Skip
    });

    assert!(refused.is_err());
    assert!(
        matches!(
            refused,
            Err(JoseError::AlgorithmMismatch { .. } | JoseError::SignatureDoesNotVerify)
        ),
        "{refused:?}"
    );
    assert!(!asked);
}

fn holder_other() -> KeyEntry {
    generated(KeyAlgorithm::EcdsaP384)
}

#[test]
fn no_holder_against_iss_comparison_is_made() {
    let holder = holder();
    let mut document = presentation(Vec::new());
    document["iss"] = json!("https://not-the-holder.example");
    let token = forge(&holder, &presentation_header(&holder), &document);

    assert!(verify_presentation(&token, &holder.public_key(), None, skip).is_ok());
}

// ── envelope shapes ───────────────────────────────────────────────────────────

fn malformed(index: usize, reason: &str) -> Result<(), JoseError> {
    Err(JoseError::MalformedEnvelope {
        index,
        reason: reason.to_owned(),
    })
}

/// Every shape error is refused, naming the envelope, and the closure is never
/// asked.
#[test]
fn a_malformed_envelope_is_refused_whatever_the_closure_would_say() {
    let holder = holder();
    let token = issuer_token(&generated(KeyAlgorithm::Ed25519));
    let good = envelope(&token);
    let cases = [
        (json!("a string"), "not a JSON object"),
        (json!(7), "not a JSON object"),
        (json!(null), "not a JSON object"),
        (
            json!({"@context": "https://www.w3.org/ns/credentials/v2", "id": "data:application/vc+jwt,x"}),
            "type is not EnvelopedVerifiableCredential",
        ),
        (
            json!({"@context": "https://www.w3.org/ns/credentials/v2", "type": "VerifiableCredential", "id": "data:application/vc+jwt,x"}),
            "type is not EnvelopedVerifiableCredential",
        ),
        (
            json!({"type": "EnvelopedVerifiableCredential", "id": "data:application/vc+jwt,x"}),
            "@context is absent",
        ),
        (
            json!({"@context": 7, "type": "EnvelopedVerifiableCredential", "id": "data:application/vc+jwt,x"}),
            "@context is absent",
        ),
        (
            json!({"@context": "c", "type": "EnvelopedVerifiableCredential"}),
            "id is not a string",
        ),
        (
            json!({"@context": "c", "type": "EnvelopedVerifiableCredential", "id": 7}),
            "id is not a string",
        ),
        (
            json!({"@context": "c", "type": "EnvelopedVerifiableCredential", "id": "https://example.org/c"}),
            "id is not a data: URL",
        ),
        (
            json!({"@context": "c", "type": "EnvelopedVerifiableCredential", "id": "data:application/vc+jwt"}),
            "id is not a data: URL",
        ),
    ];
    for (bad, reason) in cases {
        for position in [0, 1] {
            let members = if position == 0 {
                vec![bad.clone(), good.clone()]
            } else {
                vec![good.clone(), bad.clone()]
            };
            let (result, asked) = verify_members(&holder, members);

            assert_eq!(result.map(|_| ()), malformed(position, reason), "{bad}");
            assert!(!asked, "the closure was asked about {bad}");
        }
    }
}

#[test]
fn a_vc_jwt_whose_data_is_not_a_compact_jws_is_refused() {
    let holder = holder();
    let token = issuer_token(&generated(KeyAlgorithm::Ed25519));
    let (header, rest) = token.split_once('.').expect("segments");
    let (payload, _) = rest.split_once('.').expect("segments");
    // A header segment whose length is 1 modulo 4, which no base64url text has: the
    // suite's own malformed envelope is such a segment.
    let bad_header = &header[..(header.len() / 4) * 4 - 3];
    let bad = [
        header.to_owned(),
        format!("{header}.{payload}"),
        format!("{header}.{payload}."),
        format!("{header}..sig"),
        format!("{header}.{payload}.sig.extra"),
        format!("{header}.{payload}.not base64"),
        format!("{bad_header}.{payload}.AA"),
        String::new(),
    ];
    for data in bad {
        let (result, asked) = verify_members(&holder, vec![envelope(&data)]);

        assert_eq!(
            result.map(|_| ()),
            malformed(0, "the vc+jwt data is not a compact JWS"),
            "{data}"
        );
        assert!(!asked);
    }
}

/// Other media types are held only to being a `data:` URL: their content is not read.
#[test]
fn an_envelope_of_another_type_is_not_read() {
    let holder = holder();
    let members = vec![
        foreign_envelope("application/vc+sd-jwt"),
        foreign_envelope("application/vc+cose"),
        foreign_envelope("application/vc+cose;base64"),
    ];

    let (result, _) = verify_members(&holder, members);

    let verified = result.expect("verifies");
    let media: Vec<&str> = verified
        .skipped
        .iter()
        .map(|s| s.media_type.as_str())
        .collect();
    assert_eq!(
        media,
        [
            "application/vc+sd-jwt",
            "application/vc+cose",
            "application/vc+cose"
        ]
    );
}

// ── what the closure is told ──────────────────────────────────────────────────

#[test]
fn the_closure_is_called_once_per_envelope_in_order_with_what_it_needs() {
    let holder = holder();
    let issuer = generated(KeyAlgorithm::Ed25519);
    let no_kid = {
        let mut header = credential_header(&issuer);
        header.as_object_mut().expect("an object").remove("kid");
        let mut c = credential();
        c["iss"] = json!("https://issuer.example");
        forge(&issuer, &header, &c)
    };
    let mixed_case = envelope(&issuer_token(&issuer))["id"]
        .as_str()
        .expect("an id")
        .replacen(
            "data:application/vc+jwt",
            "DATA:Application/VC+JWT;charset=utf-8",
            1,
        );
    let members = vec![
        envelope(&issuer_token(&issuer)),
        foreign_envelope("Application/VC+SD-JWT"),
        envelope(&no_kid),
        json!({"@context": "c", "type": "EnvelopedVerifiableCredential", "id": mixed_case}),
    ];
    let token = sign_presentation(&presentation(members), &holder, KID, &[]).expect("signs");

    let mut calls = Vec::new();
    verify_presentation(&token, &holder.public_key(), None, |index, media, kid| {
        calls.push((index, media.to_owned(), kid.map(str::to_owned)));
        EnvelopeDecision::Skip
    })
    .expect("verifies");

    assert_eq!(
        calls,
        [
            (0, "application/vc+jwt".to_owned(), Some(KID.to_owned())),
            (1, "application/vc+sd-jwt".to_owned(), None),
            (2, "application/vc+jwt".to_owned(), None),
            (3, "application/vc+jwt".to_owned(), Some(KID.to_owned())),
        ]
    );
}

#[test]
fn a_single_envelope_without_the_array_is_one_envelope() {
    let holder = holder();
    let mut document = presentation(Vec::new());
    document["verifiableCredential"] = foreign_envelope("application/vc+cose");
    let token = sign_presentation(&document, &holder, KID, &[]).expect("signs");

    let verified = verify_presentation(&token, &holder.public_key(), None, skip).expect("verifies");

    assert_eq!(verified.skipped.len(), 1);
}

#[test]
fn a_data_url_may_carry_the_jws_in_base64() {
    let holder = holder();
    let issuer = generated(KeyAlgorithm::Ed25519);
    let token = issuer_token(&issuer);
    let encoded = base64::engine::general_purpose::STANDARD.encode(&token);
    let escaped = token.replace('.', "%2E");
    let members = vec![
        json!({"@context": "c", "type": "EnvelopedVerifiableCredential", "id": format!("data:application/vc+jwt;base64,{encoded}")}),
        json!({"@context": "c", "type": "EnvelopedVerifiableCredential", "id": format!("data:application/vc+jwt,{escaped}")}),
    ];
    let outer = sign_presentation(&presentation(members), &holder, KID, &[]).expect("signs");

    let verified = verify_presentation(&outer, &holder.public_key(), None, |_, _, _| {
        EnvelopeDecision::Verify(issuer.public_key())
    })
    .expect("verifies");

    assert_eq!(verified.verified.len(), 2);
}
