//! A `vc+sd-jwt` credential in a presentation: its shape is checked when the presentation
//! is verified, and the credential is verified under the issuer's key afterwards, in a
//! `vp+jwt` and in a `vp+sd-jwt` alike.

use base64::Engine as _;
use serde_json::{Value, json};

use dpp_crypto::jws::KeyAlgorithm;
use dpp_crypto::keystore::KeyEntry;
use dpp_crypto::sd_jwt::ClaimPath;

use super::VerifyOptions;
use super::{
    JoseError, VerifiedPresentation, sign_credential, sign_credential_sd_jwt, sign_presentation,
    sign_presentation_sd_jwt, verify_presentation, verify_presentation_sd_jwt,
};
use crate::test_support::NOT_REQUIRED;
use crate::test_support::{
    KID, credential, envelope, foreign_envelope, forge, generated, presentation,
    sd_credential_header, sd_envelope,
};

fn holder() -> KeyEntry {
    generated(KeyAlgorithm::EcdsaP256)
}

fn issuer() -> KeyEntry {
    generated(KeyAlgorithm::Ed25519)
}

/// An SD-JWT of the credential, signed by `issuer`, with its `name` concealed.
fn sd_by(issuer: &KeyEntry) -> String {
    let name = ClaimPath::from_json(&json!(["credentialSubject", "name"])).expect("a path");
    sign_credential_sd_jwt(&credential(), issuer, KID, &[], &[name], None)
        .expect("signs")
        .serialise()
}

/// A presentation of `members`, signed as a `vp+jwt` by `holder` and verified.
fn present_jwt(holder: &KeyEntry, members: Vec<Value>) -> Result<VerifiedPresentation, JoseError> {
    let token = sign_presentation(&presentation(members), holder, KID, &[]).expect("signs");
    verify_presentation(&token, &holder.public_key(), VerifyOptions::new())
}

/// The same, signed as a `vp+sd-jwt`.
fn present_sd(holder: &KeyEntry, members: Vec<Value>) -> Result<VerifiedPresentation, JoseError> {
    let token = sign_presentation_sd_jwt(&presentation(members), holder, KID, &[], &[], None)
        .expect("signs")
        .serialise();
    verify_presentation_sd_jwt(
        &token,
        &holder.public_key(),
        VerifyOptions::new(),
        &NOT_REQUIRED,
    )
}

type Present = fn(&KeyEntry, Vec<Value>) -> Result<VerifiedPresentation, JoseError>;
const BOTH: [(&str, Present); 2] = [("vp+jwt", present_jwt), ("vp+sd-jwt", present_sd)];

#[test]
fn a_vc_sd_jwt_envelope_is_listed_with_its_kid_and_verifies_under_the_issuers_key() {
    let (holder, issuer) = (holder(), issuer());
    for (outer, present) in BOTH {
        let verified = present(&holder, vec![sd_envelope(&sd_by(&issuer))]).expect("verifies");

        let listed: Vec<_> = verified
            .envelopes()
            .iter()
            .map(|e| (e.index, e.media_type.as_str(), e.kid.as_deref()))
            .collect();
        assert_eq!(listed, [(0, "application/vc+sd-jwt", Some(KID))], "{outer}");
        let credential = verified.envelopes()[0]
            .verify(&issuer.public_key(), VerifyOptions::new(), &NOT_REQUIRED)
            .expect("verifies");
        assert_eq!(credentials_name(&credential), Some("Example"), "{outer}");
    }
}

fn credentials_name(credential: &Value) -> Option<&str> {
    credential["credentialSubject"]["name"].as_str()
}

#[test]
fn verify_credentials_verifies_a_vc_jwt_and_a_vc_sd_jwt_each_under_its_own_key() {
    let (holder, jwt_issuer, sd_issuer) = (holder(), issuer(), generated(KeyAlgorithm::EcdsaP384));
    let jwt = sign_credential(&credential(), &jwt_issuer, KID, &[]).expect("signs");
    for (outer, present) in BOTH {
        let verified = present(
            &holder,
            vec![envelope(&jwt), sd_envelope(&sd_by(&sd_issuer))],
        )
        .expect("verifies");

        let credentials = verified
            .verify_credentials(VerifyOptions::new(), &NOT_REQUIRED, |e| {
                Some(if e.media_type == "application/vc+jwt" {
                    jwt_issuer.public_key()
                } else {
                    sd_issuer.public_key()
                })
            })
            .expect("verifies");

        assert_eq!(credentials, [credential(), credential()], "{outer}");
    }
}

#[test]
fn an_envelope_is_verified_strictly_and_a_failure_names_it() {
    let (holder, issuer, other) = (holder(), issuer(), issuer());
    for (outer, present) in BOTH {
        let verified = present(&holder, vec![sd_envelope(&sd_by(&issuer))]).expect("verifies");

        let refused = verified.verify_credentials(VerifyOptions::new(), &NOT_REQUIRED, |_| {
            Some(other.public_key())
        });

        assert_eq!(
            refused,
            Err(JoseError::CredentialRefused {
                index: 0,
                source: Box::new(JoseError::SignatureDoesNotVerify)
            }),
            "{outer}"
        );
        assert_eq!(
            verified.envelopes()[0]
                .verify(&other.public_key(), VerifyOptions::new(), &NOT_REQUIRED)
                .map(|_| ()),
            Err(JoseError::CredentialRefused {
                index: 0,
                source: Box::new(JoseError::SignatureDoesNotVerify)
            }),
            "{outer}"
        );
    }
}

/// An envelope under `now` is held to the credential's own `exp`.
#[test]
fn an_envelope_is_held_to_the_now_given() {
    let (holder, issuer) = (holder(), issuer());
    let mut document = credential();
    document["exp"] = json!(1_000);
    let token = {
        let key = &issuer;
        crate::test_support::forge_sd(key, &sd_credential_header(key), &document, &[])
    };
    let verified = present_jwt(&holder, vec![sd_envelope(&token)]).expect("verifies");

    assert!(
        verified.envelopes()[0]
            .verify(&issuer.public_key(), VerifyOptions::new(), &NOT_REQUIRED)
            .is_ok()
    );
    assert_eq!(
        verified.envelopes()[0]
            .verify(
                &issuer.public_key(),
                VerifyOptions::new().at(1_000),
                &NOT_REQUIRED
            )
            .map(|_| ()),
        Err(JoseError::CredentialRefused {
            index: 0,
            source: Box::new(JoseError::Expired)
        })
    );
}

/// RFC 9901 §7.3 steps 3 and 4: when Key Binding is not required, a token that carries a
/// Key Binding JWT is processed as an SD-JWT, and the Key Binding JWT is not read. This one
/// is not even a valid Key Binding JWT, and is ignored.
#[test]
fn an_envelope_with_a_key_binding_jwt_is_verified_as_an_sd_jwt_when_binding_is_not_required() {
    let (holder, issuer) = (holder(), issuer());
    let kb = forge(
        &issuer,
        &json!({"alg": "EdDSA", "typ": "kb+jwt"}),
        &json!({}),
    );
    let with_kb = format!("{}{kb}", sd_by(&issuer));
    let verified = present_jwt(&holder, vec![sd_envelope(&with_kb)]).expect("verifies");

    assert!(
        verified.envelopes()[0]
            .verify(&issuer.public_key(), VerifyOptions::new(), &NOT_REQUIRED)
            .is_ok()
    );
}

// ── the shape of the data ─────────────────────────────────────────────────────

/// The presentation is verified by judging its envelopes, so data that is not an SD-JWT does
/// not stop it. The credential is refused when it is verified, and the refusal says why.
#[test]
fn a_vc_sd_jwt_envelope_whose_data_is_not_an_sd_jwt_is_refused_when_its_credential_is_verified() {
    let holder = holder();
    let issuer = issuer();
    let sd = sd_by(&issuer);
    let without_tilde = sd.trim_end_matches('~').to_owned();
    let no_dots = "abc~".to_owned();
    for data in [
        "opaque".to_owned(),
        String::new(),
        without_tilde,
        no_dots,
        "a.b.c".to_owned(),
        "a.b.!~".to_owned(),
        "%zz".to_owned(),
    ] {
        let member = sd_envelope(&data);
        for (outer, present) in BOTH {
            let verified = present(&holder, vec![member.clone()])
                .unwrap_or_else(|e| panic!("{outer} {data:?}: {e}"));
            let refused = verified.envelopes()[0]
                .verify(&issuer.public_key(), VerifyOptions::new(), &NOT_REQUIRED)
                .expect_err("refused");
            assert!(
                matches!(refused, JoseError::CredentialRefused { index: 0, .. }),
                "{outer} {data:?}: {refused:?}"
            );
        }
    }
    // Data that cannot be decoded at all says so, and data that decodes to text says what it is not.
    let verified = present_jwt(&holder, vec![sd_envelope("%zz")]).expect("verifies");
    assert!(matches!(
        verified.envelopes()[0].verify(&issuer.public_key(), VerifyOptions::new(), &NOT_REQUIRED),
        Err(JoseError::CredentialRefused { source, .. })
            if matches!(*source, JoseError::UnreadableEnvelopeData { .. })
    ));
    let verified = present_jwt(&holder, vec![sd_envelope("opaque")]).expect("verifies");
    assert!(matches!(
        verified.envelopes()[0].verify(&issuer.public_key(), VerifyOptions::new(), &NOT_REQUIRED),
        Err(JoseError::CredentialRefused { source, .. })
            if matches!(*source, JoseError::MalformedSdJwt { .. })
    ));
}

/// A `data:` URL's data is percent-decoded, and base64-decoded first under `;base64`, as
/// for a `vc+jwt`.
#[test]
fn a_vc_sd_jwt_envelope_may_be_percent_encoded_or_base64() {
    let (holder, issuer) = (holder(), issuer());
    let sd = sd_by(&issuer);
    let percent = sd.replace('~', "%7E");
    let base64 = base64::engine::general_purpose::STANDARD.encode(&sd);
    let id = |media: &str, data: &str| {
        json!({
            "@context": "https://www.w3.org/ns/credentials/v2",
            "type": "EnvelopedVerifiableCredential",
            "id": format!("data:{media},{data}"),
        })
    };

    for member in [
        id("application/vc+sd-jwt", &percent),
        id("application/vc+sd-jwt;base64", &base64),
        id("APPLICATION/VC+SD-JWT;charset=utf-8", &sd),
    ] {
        let verified = present_jwt(&holder, vec![member]).expect("verifies");
        assert_eq!(verified.envelopes()[0].media_type, "application/vc+sd-jwt");
        assert_eq!(verified.envelopes()[0].kid.as_deref(), Some(KID));
        assert!(
            verified.envelopes()[0]
                .verify(&issuer.public_key(), VerifyOptions::new(), &NOT_REQUIRED)
                .is_ok()
        );
    }
}

#[test]
fn a_vc_sd_jwt_envelope_without_a_kid_lists_none() {
    let (holder, issuer) = (holder(), issuer());
    let mut header = sd_credential_header(&issuer);
    header.as_object_mut().expect("an object").remove("kid");
    let mut document = credential();
    document["iss"] = json!("https://issuer.example");
    let token = crate::test_support::forge_sd(&issuer, &header, &document, &[]);

    let verified = present_jwt(&holder, vec![sd_envelope(&token)]).expect("verifies");

    assert_eq!(verified.envelopes()[0].kid, None);
    assert!(
        verified.envelopes()[0]
            .verify(&issuer.public_key(), VerifyOptions::new(), &NOT_REQUIRED)
            .is_ok()
    );
}

/// Only a `vc+jwt`, a `vc+sd-jwt` and a `vc+cose` are verified. The others are listed as they
/// were, and refused when the credentials are verified.
#[test]
fn another_format_is_still_listed_and_still_not_verified() {
    let (holder, issuer) = (holder(), issuer());
    let members = vec![sd_envelope(&sd_by(&issuer)), foreign_envelope("text/plain")];

    let verified = present_jwt(&holder, members).expect("verifies");

    assert_eq!(
        verified.verify_credentials(VerifyOptions::new(), &NOT_REQUIRED, |_| Some(
            issuer.public_key()
        )),
        Err(JoseError::UnsupportedEnvelopeFormat {
            index: 1,
            media_type: "text/plain".to_owned(),
        })
    );
}

#[test]
fn no_key_for_a_vc_sd_jwt_envelope_names_its_kid() {
    let (holder, issuer) = (holder(), issuer());
    let verified = present_jwt(&holder, vec![sd_envelope(&sd_by(&issuer))]).expect("verifies");

    assert_eq!(
        verified.verify_credentials(VerifyOptions::new(), &NOT_REQUIRED, |_| None),
        Err(JoseError::CredentialKeyUnknown {
            index: 0,
            kid: Some(KID.to_owned()),
        })
    );
}
