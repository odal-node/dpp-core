//! The audience a verifier identifies as, RFC 7519 §4.1.3: "If the principal processing the
//! claim does not identify itself with a value in the "aud" claim when this claim is
//! present, then the JWT MUST be rejected."
//!
//! Every form of token meets the same rule: a `vc+jwt`, a `vp+jwt`, the processed payload of a
//! `vc+sd-jwt` and of a `vp+sd-jwt`, and the credential in an envelope of a presentation,
//! verified as a `vc+jwt` and as a `vc+sd-jwt`. Each test loops over all of them.

use serde_json::{Value, json};

use dpp_crypto::jws::KeyAlgorithm;
use dpp_crypto::keystore::KeyEntry;

use super::{
    JoseError, VerifyOptions, sign_presentation, verify_credential, verify_credential_sd_jwt,
    verify_presentation, verify_presentation_sd_jwt,
};
use crate::test_support::{
    KID, NOT_REQUIRED, credential, credential_header, envelope, forge, forge_sd, generated,
    presentation, presentation_header, sd_credential_header, sd_envelope, sd_presentation_header,
};

const AUDIENCE: &str = "https://verifier.example";
const OTHER: &str = "https://other.example";

#[derive(Debug, Clone, Copy)]
enum Form {
    Vc,
    Vp,
    SdVc,
    SdVp,
    EnvelopedVc,
    EnvelopedSdVc,
}

const FORMS: [Form; 6] = [
    Form::Vc,
    Form::Vp,
    Form::SdVc,
    Form::SdVp,
    Form::EnvelopedVc,
    Form::EnvelopedSdVc,
];

fn key() -> KeyEntry {
    generated(KeyAlgorithm::EcdsaP256)
}

/// A credential with `aud` as given, or none.
fn credential_with(aud: Option<&Value>) -> Value {
    let mut document = credential();
    if let Some(aud) = aud {
        document["aud"] = aud.clone();
    }
    document
}

/// A presentation with `aud` as given, or none.
fn presentation_with(aud: Option<&Value>, envelopes: Vec<Value>) -> Value {
    let mut document = presentation(envelopes);
    if let Some(aud) = aud {
        document["aud"] = aud.clone();
    }
    document
}

/// The token of `form` whose payload carries `aud` (for an envelope, the credential's), signed
/// by `key`, verified under `options`. What a credential in an envelope was refused for is
/// what comes back, without the envelope around it.
fn verify(
    form: Form,
    key: &KeyEntry,
    aud: Option<&Value>,
    options: VerifyOptions<'_>,
) -> Result<(), JoseError> {
    let public = key.public_key();
    match form {
        Form::Vc => {
            let token = forge(key, &credential_header(key), &credential_with(aud));
            verify_credential(&token, &public, options).map(|_| ())
        }
        Form::Vp => {
            let token = forge(
                key,
                &presentation_header(key),
                &presentation_with(aud, vec![]),
            );
            verify_presentation(&token, &public, options).map(|_| ())
        }
        Form::SdVc => {
            let token = forge_sd(key, &sd_credential_header(key), &credential_with(aud), &[]);
            verify_credential_sd_jwt(&token, &public, options, &NOT_REQUIRED).map(|_| ())
        }
        Form::SdVp => {
            let token = forge_sd(
                key,
                &sd_presentation_header(key),
                &presentation_with(aud, vec![]),
                &[],
            );
            verify_presentation_sd_jwt(&token, &public, options, &NOT_REQUIRED).map(|_| ())
        }
        Form::EnvelopedVc | Form::EnvelopedSdVc => {
            let envelope = if matches!(form, Form::EnvelopedVc) {
                envelope(&forge(key, &credential_header(key), &credential_with(aud)))
            } else {
                sd_envelope(&forge_sd(
                    key,
                    &sd_credential_header(key),
                    &credential_with(aud),
                    &[],
                ))
            };
            let outer =
                sign_presentation(&presentation(vec![envelope]), key, KID, &[]).expect("signs");
            let verified =
                verify_presentation(&outer, &public, VerifyOptions::new()).expect("verifies");
            match verified.verify_credentials(options, &NOT_REQUIRED, |_| Some(public.clone())) {
                Ok(_) => Ok(()),
                Err(JoseError::CredentialRefused { source, .. }) => Err(*source),
                Err(other) => Err(other),
            }
        }
    }
}

/// The control: a token that carries no `aud` is not affected, whatever audience the caller
/// gives or does not give.
#[test]
fn a_token_with_no_aud_is_accepted_with_or_without_an_audience() {
    let key = key();
    for form in FORMS {
        assert_eq!(
            verify(form, &key, None, VerifyOptions::new()),
            Ok(()),
            "{form:?}"
        );
        assert_eq!(
            verify(
                form,
                &key,
                None,
                VerifyOptions::new().for_audience(AUDIENCE)
            ),
            Ok(()),
            "{form:?}"
        );
    }
}

/// §4.1.3: the value is "an array of case-sensitive strings", or, "when the JWT has one
/// audience", "a single case-sensitive string".
#[test]
fn a_token_whose_aud_names_the_audience_is_accepted() {
    let key = key();
    let options = VerifyOptions::new().for_audience(AUDIENCE);
    for form in FORMS {
        for aud in [
            json!(AUDIENCE),
            json!([AUDIENCE]),
            json!([OTHER, AUDIENCE]),
            json!([AUDIENCE, OTHER]),
        ] {
            assert_eq!(
                verify(form, &key, Some(&aud), options),
                Ok(()),
                "{form:?} {aud}"
            );
        }
    }
}

#[test]
fn a_token_whose_aud_names_another_audience_is_refused() {
    let key = key();
    let options = VerifyOptions::new().for_audience(AUDIENCE);
    for form in FORMS {
        for aud in [
            json!(OTHER),
            json!([OTHER]),
            json!([OTHER, "https://third.example"]),
            // Case-sensitive, and an exact match: not a prefix, not a suffix, not a URL's
            // other spelling.
            json!("HTTPS://VERIFIER.EXAMPLE"),
            json!("https://verifier.example/"),
            json!([]),
        ] {
            assert_eq!(
                verify(form, &key, Some(&aud), options),
                Err(JoseError::AudienceMismatch {
                    audience: Some(AUDIENCE.to_owned())
                }),
                "{form:?} {aud}"
            );
        }
    }
}

/// A caller that gives no audience identifies itself with nothing, so no `aud` can name it.
#[test]
fn a_token_that_carries_aud_is_refused_when_no_audience_is_given() {
    let key = key();
    for form in FORMS {
        for aud in [json!(AUDIENCE), json!([AUDIENCE]), json!([])] {
            assert_eq!(
                verify(form, &key, Some(&aud), VerifyOptions::new()),
                Err(JoseError::AudienceMismatch { audience: None }),
                "{form:?} {aud}"
            );
        }
    }
}

/// An `aud` of another shape is not read as no `aud`: a `null` or a number would otherwise
/// switch the rule off. An array is held to it entry by entry.
#[test]
fn an_aud_that_is_not_a_string_or_an_array_of_strings_is_refused() {
    let key = key();
    for form in FORMS {
        for aud in [
            json!(7),
            json!(null),
            json!(true),
            json!({"aud": AUDIENCE}),
            json!([1]),
            json!([null]),
            json!([AUDIENCE, 1]),
            json!([[AUDIENCE]]),
        ] {
            for options in [
                VerifyOptions::new(),
                VerifyOptions::new().for_audience(AUDIENCE),
            ] {
                assert_eq!(
                    verify(form, &key, Some(&aud), options),
                    Err(JoseError::MalformedAudience),
                    "{form:?} {aud}"
                );
            }
        }
    }
}

/// The audience is compared with the claim of the token it is checked on. A presentation
/// that names the audience says nothing for the credentials in it, which are verified with
/// their own options.
#[test]
fn the_audience_of_a_presentation_is_not_the_audience_of_its_credentials() {
    let key = key();
    let public = key.public_key();
    let inner = forge(
        &key,
        &credential_header(&key),
        &credential_with(Some(&json!(OTHER))),
    );
    let outer = forge(
        &key,
        &presentation_header(&key),
        &presentation_with(Some(&json!(AUDIENCE)), vec![envelope(&inner)]),
    );
    let options = VerifyOptions::new().for_audience(AUDIENCE);

    let verified = verify_presentation(&outer, &public, options).expect("the outer token names it");

    assert_eq!(
        verified.envelopes()[0].verify(&public, options, &NOT_REQUIRED),
        Err(JoseError::CredentialRefused {
            index: 0,
            source: Box::new(JoseError::AudienceMismatch {
                audience: Some(AUDIENCE.to_owned())
            })
        })
    );
    assert!(
        verified.envelopes()[0]
            .verify(
                &public,
                VerifyOptions::new().for_audience(OTHER),
                &NOT_REQUIRED
            )
            .is_ok()
    );
}
