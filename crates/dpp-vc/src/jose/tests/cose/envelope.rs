//! A `vc+cose` credential in a presentation: verified inside each kind of presentation, read
//! as a `data:` URL, and refused where Key Binding is required (VC-JOSE-COSE §3.3.1).
//!
//! Messages a signer would never write are forged with a genuine signature
//! ([`super::super::cose::common`]), so a refusal is the rule it names and not a signature that never
//! verified.

use serde_json::{Value, json};

use dpp_crypto::cose::CoseError;
use dpp_crypto::sd_jwt::KeyBindingPolicy;

use super::super::super::{
    JoseError, KeyBindingRequirement, VerifiedPresentation, VerifyOptions, sign_credential_cose,
    sign_presentation, sign_presentation_cose, sign_presentation_sd_jwt, verify_presentation,
    verify_presentation_cose, verify_presentation_sd_jwt,
};
use super::super::cose::common::{
    ALGORITHMS, AUDIENCE, claim, cwt, forged_credential, holder, issuer, tstr, uint,
};
use crate::test_support::{
    KID, NOT_REQUIRED, cose_envelope, credential, presentation, public_key, signing_key,
};

// ── envelopes ─────────────────────────────────────────────────────────────────

fn signed_vc() -> Vec<u8> {
    sign_credential_cose(&credential(), &issuer(), KID, &[]).expect("signs")
}

fn present_jwt(members: Vec<Value>) -> Result<VerifiedPresentation, JoseError> {
    let token = sign_presentation(&presentation(members), &holder(), KID, &[]).expect("signs");
    verify_presentation(&token, &public_key("p384"), VerifyOptions::new())
}

fn present_sd(members: Vec<Value>) -> Result<VerifiedPresentation, JoseError> {
    let token = sign_presentation_sd_jwt(&presentation(members), &holder(), KID, &[], &[], None)
        .expect("signs")
        .serialise();
    verify_presentation_sd_jwt(
        &token,
        &public_key("p384"),
        VerifyOptions::new(),
        &NOT_REQUIRED,
    )
}

fn present_cose(members: Vec<Value>) -> Result<VerifiedPresentation, JoseError> {
    let message =
        sign_presentation_cose(&presentation(members), &holder(), KID, &[]).expect("signs");
    verify_presentation_cose(&message, &public_key("p384"), VerifyOptions::new())
}

type Present = fn(Vec<Value>) -> Result<VerifiedPresentation, JoseError>;
const OUTERS: [(&str, Present); 3] = [
    ("vp+jwt", present_jwt),
    ("vp+sd-jwt", present_sd),
    ("vp+cose", present_cose),
];

/// A `vc+cose` envelope verifies inside each kind of presentation, one at a time and all
/// together, under the issuer's key.
#[test]
fn a_vc_cose_envelope_verifies_inside_every_kind_of_presentation() {
    for (outer, present) in OUTERS {
        let verified =
            present(vec![cose_envelope(&signed_vc())]).unwrap_or_else(|e| panic!("{outer}: {e}"));

        let listed: Vec<_> = verified
            .envelopes()
            .iter()
            .map(|e| (e.index, e.media_type.as_str(), e.kid.as_deref()))
            .collect();
        assert_eq!(listed, [(0, "application/vc+cose", None)], "{outer}");
        assert_eq!(
            verified.envelopes()[0].verify(
                &public_key("p256"),
                VerifyOptions::new(),
                &NOT_REQUIRED
            ),
            Ok(credential()),
            "{outer}"
        );
        assert_eq!(
            verified.verify_credentials(VerifyOptions::new(), &NOT_REQUIRED, |_| Some(public_key(
                "p256"
            ))),
            Ok(vec![credential()]),
            "{outer}"
        );
        // Under the wrong key it is the credential that is refused, and the envelope is named.
        assert_eq!(
            verified.envelopes()[0].verify(
                &public_key("p384"),
                VerifyOptions::new(),
                &NOT_REQUIRED
            ),
            Err(JoseError::CredentialRefused {
                index: 0,
                source: Box::new(JoseError::Cose(CoseError::AlgorithmMismatch)),
            }),
            "{outer}"
        );
    }
}

/// What is inside an envelope is not read when the presentation is verified, whatever it is.
/// The credential is refused when it is verified, and the refusal says why.
#[test]
fn a_vc_cose_envelope_that_is_not_readable_is_refused_when_its_credential_is_verified() {
    let id = |data: &str| {
        json!({
            "@context": "https://www.w3.org/ns/credentials/v2",
            "type": "EnvelopedVerifiableCredential",
            "id": format!("data:application/vc+cose;base64,{data}"),
        })
    };
    let not_cose = base64_of(b"this is not a COSE_Sign1 message");
    for (outer, present) in OUTERS {
        for (data, reason) in [
            ("a-b_c", "unreadable"),
            ("A", "unreadable"),
            ("%%%", "unreadable"),
            (not_cose.as_str(), "not cose"),
            ("", "not cose"),
        ] {
            let verified =
                present(vec![id(data)]).unwrap_or_else(|e| panic!("{outer} {data}: {e}"));
            let refused = verified.envelopes()[0]
                .verify(&public_key("p256"), VerifyOptions::new(), &NOT_REQUIRED)
                .expect_err("refused");
            let JoseError::CredentialRefused { index: 0, source } = refused else {
                panic!("{outer} {data}: {refused:?}");
            };
            match reason {
                "unreadable" => assert!(
                    matches!(*source, JoseError::UnreadableEnvelopeData { .. }),
                    "{outer} {data}: {source:?}"
                ),
                _ => assert!(
                    matches!(*source, JoseError::Cose(_)),
                    "{outer} {data}: {source:?}"
                ),
            }
        }
    }
}

fn base64_of(bytes: &[u8]) -> String {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// The data is read as the WHATWG Infra Standard's forgiving-base64 decode reads it, so the same
/// message written padded, unpadded, wrapped in lines, or percent-encoded without `;base64`
/// verifies alike.
#[test]
fn a_vc_cose_envelope_may_be_written_in_any_encoding_a_data_url_reader_accepts() {
    let message = signed_vc();
    let padded = base64_of(&message);
    let unpadded = padded.trim_end_matches('=').to_owned();
    let wrapped: String = padded
        .as_bytes()
        .chunks(60)
        .map(|line| format!("{}\r\n", std::str::from_utf8(line).unwrap()))
        .collect();
    let percent: String = message.iter().map(|b| format!("%{b:02X}")).collect();
    let id = |media: &str, data: &str| {
        json!({
            "@context": "https://www.w3.org/ns/credentials/v2",
            "type": "EnvelopedVerifiableCredential",
            "id": format!("data:{media},{data}"),
        })
    };

    for member in [
        id("application/vc+cose;base64", &padded),
        id("application/vc+cose;base64", &unpadded),
        id("application/vc+cose;base64", &format!("{unpadded}\n")),
        id(
            "application/vc+cose;base64",
            &wrapped.replace("\r\n", "%0D%0A"),
        ),
        id("application/vc+cose", &percent),
        id("APPLICATION/VC+COSE;charset=binary;base64", &padded),
    ] {
        let verified = present_jwt(vec![member.clone()]).expect("verifies");
        assert_eq!(
            verified.envelopes()[0].verify(
                &public_key("p256"),
                VerifyOptions::new(),
                &NOT_REQUIRED
            ),
            Ok(credential()),
            "{member}"
        );
    }
}

/// A credential's `aud` is checked against the audience of the call that verifies it, as for the
/// other forms, and the time claims against its `now`.
#[test]
fn the_options_apply_to_the_credential_in_an_envelope() {
    let message = forged_credential(
        &credential(),
        &cwt(vec![
            claim(3, tstr("https://other.example")),
            claim(4, uint(100)),
        ]),
    );
    let verified = present_jwt(vec![cose_envelope(&message)]).expect("verifies");
    let envelope = &verified.envelopes()[0];
    let key = public_key("p256");
    let refused = |source: JoseError| JoseError::CredentialRefused {
        index: 0,
        source: Box::new(source),
    };

    assert_eq!(
        envelope.verify(&key, VerifyOptions::new(), &NOT_REQUIRED),
        Err(refused(JoseError::AudienceMismatch { audience: None }))
    );
    assert_eq!(
        envelope.verify(
            &key,
            VerifyOptions::new()
                .for_audience("https://other.example")
                .at(100),
            &NOT_REQUIRED
        ),
        Err(refused(JoseError::Expired))
    );
    assert!(
        envelope
            .verify(
                &key,
                VerifyOptions::new()
                    .for_audience("https://other.example")
                    .at(99),
                &NOT_REQUIRED
            )
            .is_ok()
    );
}

/// A `vc+cose` cannot carry a Key Binding JWT, so where Key Binding is required it is refused,
/// as a `vc+jwt` envelope is.
#[test]
fn a_vc_cose_envelope_is_refused_where_key_binding_is_required() {
    let required = KeyBindingRequirement::Required(KeyBindingPolicy {
        audience: AUDIENCE.to_owned(),
        nonce: "n".to_owned(),
        now: 1_000,
        max_age: 300,
        leeway: 60,
    });
    let verified = present_jwt(vec![cose_envelope(&signed_vc())]).expect("verifies");
    let expected = Err(JoseError::CredentialRefused {
        index: 0,
        source: Box::new(JoseError::KeyBindingMissing),
    });

    assert_eq!(
        verified.envelopes()[0]
            .verify(&public_key("p256"), VerifyOptions::new(), &required)
            .map(|_| ()),
        expected
    );
    assert_eq!(
        verified
            .verify_credentials(VerifyOptions::new(), &required, |_| Some(public_key(
                "p256"
            )))
            .map(|_| ()),
        expected
    );
}

/// The envelope is judged for what it is: its `type`, its `@context` and a `data:` URL `id`.
#[test]
fn an_envelope_that_is_not_an_envelope_is_refused_by_the_presentation() {
    for (member, reason) in [
        (
            json!("data:application/vc+cose;base64,AAAA"),
            "not a JSON object",
        ),
        (
            json!({"@context": "https://www.w3.org/ns/credentials/v2", "type": "Other", "id": "data:application/vc+cose;base64,AA"}),
            "type is not EnvelopedVerifiableCredential",
        ),
        (
            json!({"type": "EnvelopedVerifiableCredential", "id": "data:application/vc+cose;base64,AA"}),
            "@context is absent",
        ),
        (
            json!({"@context": "https://www.w3.org/ns/credentials/v2", "type": "EnvelopedVerifiableCredential", "id": "https://example.org/vc"}),
            "id is not a data: URL",
        ),
    ] {
        for (outer, present) in OUTERS {
            assert_eq!(
                present(vec![member.clone()]).map(|_| ()),
                Err(JoseError::MalformedEnvelope {
                    index: 0,
                    reason: reason.to_owned()
                }),
                "{outer}"
            );
        }
    }
}

#[test]
fn an_envelope_of_another_media_type_is_still_not_verified() {
    let verified = present_cose(vec![crate::test_support::foreign_envelope(
        "application/vc+ld+json",
    )])
    .expect("verifies");

    assert_eq!(
        verified.verify_credentials(VerifyOptions::new(), &NOT_REQUIRED, |_| Some(public_key(
            "p256"
        ))),
        Err(JoseError::UnsupportedEnvelopeFormat {
            index: 0,
            media_type: "application/vc+ld+json".to_owned()
        })
    );
}

/// Every algorithm's credential verifies in an envelope under its own key, and the key of
/// another algorithm is refused for its algorithm.
#[test]
fn an_envelope_verifies_under_each_algorithms_key_and_no_other() {
    for algorithm in ALGORITHMS {
        let signed =
            sign_credential_cose(&credential(), &signing_key(algorithm), KID, &[]).unwrap();
        let verified = present_cose(vec![cose_envelope(&signed)]).expect("verifies");
        for other in ALGORITHMS {
            let result = verified.envelopes()[0].verify(
                &public_key(other),
                VerifyOptions::new(),
                &NOT_REQUIRED,
            );
            assert_eq!(
                result.is_ok(),
                other == algorithm,
                "{algorithm} under {other}"
            );
        }
    }
}
