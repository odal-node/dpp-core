//! The `type` and `@context` of an envelope in a presentation: what the shape check
//! accepts and refuses, before `verify_presentation` lists any envelope.
//!
//! The envelopes here are of a media type this crate does not read, so only the shape
//! is under test. The shape rules that were there before (`@context` absent, `id` not a
//! `data:` URL, and the rest) are in `presentation_tests`.

use serde_json::{Value, json};

use dpp_crypto::jws::KeyAlgorithm;

use super::VerifyOptions;
use super::{JoseError, sign_presentation, verify_presentation};
use crate::test_support::{KID, generated, presentation};

const V2: &str = "https://www.w3.org/ns/credentials/v2";

/// An envelope of a media type this crate does not read, with `type` and `@context` as
/// given.
fn envelope_with(type_: Value, context: Value) -> Value {
    json!({
        "@context": context,
        "type": type_,
        "id": "data:application/vc+cose,opaque",
    })
}

/// Verify a presentation holding `member` alone. An accepted envelope is listed once.
fn verify_alone(member: Value) -> Result<(), JoseError> {
    let holder = generated(KeyAlgorithm::Ed25519);
    let token = sign_presentation(&presentation(vec![member]), &holder, KID, &[]).expect("signs");
    verify_presentation(&token, &holder.public_key(), VerifyOptions::new())
        .map(|verified| assert_eq!(verified.envelopes().len(), 1))
}

fn malformed(reason: &str) -> Result<(), JoseError> {
    Err(JoseError::MalformedEnvelope {
        index: 0,
        reason: reason.to_owned(),
    })
}

#[test]
fn an_envelope_type_is_the_term_or_a_one_item_array_holding_it() {
    for type_ in [
        json!("EnvelopedVerifiableCredential"),
        json!(["EnvelopedVerifiableCredential"]),
    ] {
        let result = verify_alone(envelope_with(type_.clone(), json!(V2)));

        assert_eq!(result, Ok(()), "{type_}");
    }
}

#[test]
fn an_envelope_type_that_is_a_longer_array_or_another_term_is_refused() {
    for type_ in [
        json!(["EnvelopedVerifiableCredential", "Other"]),
        json!(["Other", "EnvelopedVerifiableCredential"]),
        json!(["VerifiableCredential"]),
        json!([]),
        json!([7]),
        json!("VerifiableCredential"),
    ] {
        let result = verify_alone(envelope_with(type_.clone(), json!(V2)));

        assert_eq!(
            result,
            malformed("type is not EnvelopedVerifiableCredential"),
            "{type_}"
        );
    }
}

#[test]
fn an_empty_envelope_context_array_is_refused() {
    let result = verify_alone(envelope_with(
        json!("EnvelopedVerifiableCredential"),
        json!([]),
    ));

    assert_eq!(result, malformed("@context is an empty array"));
}

#[test]
fn an_envelope_context_item_that_is_neither_a_string_nor_an_object_is_refused() {
    for context in [
        json!([7]),
        json!([V2, 7]),
        json!([V2, null]),
        json!([V2, ["nested"]]),
    ] {
        let result = verify_alone(envelope_with(
            json!("EnvelopedVerifiableCredential"),
            context.clone(),
        ));

        assert_eq!(
            result,
            malformed("@context has an item that is neither a string nor an object"),
            "{context}"
        );
    }
}

#[test]
fn an_envelope_context_of_strings_and_objects_is_accepted() {
    for context in [
        json!([V2, "https://www.w3.org/ns/credentials/examples/v2"]),
        json!([V2, {"@vocab": "https://example.org/terms#"}]),
        json!({"@vocab": "https://example.org/terms#"}),
    ] {
        let result = verify_alone(envelope_with(
            json!("EnvelopedVerifiableCredential"),
            context.clone(),
        ));

        assert_eq!(result, Ok(()), "{context}");
    }
}
