//! The caller's decision for each envelope: verify, skip, or nothing known.
//!
//! Nothing a presentation envelops passes unverified without being reported, and an
//! envelope is refused unless the caller verifies it or skips it by name.

use serde_json::{Value, json};

use dpp_crypto::jws::KeyAlgorithm;
use dpp_crypto::keystore::KeyEntry;

use super::test_fixtures::{
    KID, credential, credential_header, envelope, foreign_envelope, forge, generated, presentation,
};
use super::{
    EnvelopeDecision, JoseError, SkippedEnvelope, VerifiedPresentation, sign_credential,
    sign_presentation, verify_presentation,
};

fn holder() -> KeyEntry {
    generated(KeyAlgorithm::EcdsaP256)
}

fn issuer() -> KeyEntry {
    generated(KeyAlgorithm::Ed25519)
}

fn present(
    holder: &KeyEntry,
    members: Vec<Value>,
    now: Option<i64>,
    decide: impl FnMut(usize, &str, Option<&str>) -> EnvelopeDecision,
) -> Result<VerifiedPresentation, JoseError> {
    let token = sign_presentation(&presentation(members), holder, KID, &[]).expect("signs");
    verify_presentation(&token, &holder.public_key(), now, decide)
}

fn signed_by(issuer: &KeyEntry) -> Value {
    envelope(&sign_credential(&credential(), issuer, KID, &[]).expect("signs"))
}

#[test]
fn an_envelope_nobody_decided_about_refuses_the_presentation_and_names_it() {
    let (holder, issuer) = (holder(), issuer());
    let members = vec![foreign_envelope("application/vc+cose"), signed_by(&issuer)];

    let refused = present(&holder, members, None, |index, _, _| {
        if index == 0 {
            EnvelopeDecision::Skip
        } else {
            EnvelopeDecision::Unknown
        }
    });

    assert_eq!(
        refused.map(|_| ()),
        Err(JoseError::EnvelopeNotDecided {
            index: 1,
            media_type: "application/vc+jwt".to_owned(),
            kid: Some(KID.to_owned()),
        })
    );
}

#[test]
fn an_envelope_of_another_format_cannot_be_verified_here() {
    let holder = holder();
    let key = issuer().public_key();

    for media_type in ["application/vc+sd-jwt", "application/vc+cose", "text/plain"] {
        let refused = present(
            &holder,
            vec![foreign_envelope(media_type)],
            None,
            |_, _, _| EnvelopeDecision::Verify(key.clone()),
        );

        assert_eq!(
            refused.map(|_| ()),
            Err(JoseError::UnsupportedEnvelopeFormat {
                index: 0,
                media_type: media_type.to_owned(),
            }),
            "{media_type}"
        );
    }
}

#[test]
fn a_skipped_envelope_is_reported_with_its_index_media_type_and_kid() {
    let (holder, issuer) = (holder(), issuer());
    let members = vec![foreign_envelope("application/vc+cose"), signed_by(&issuer)];

    let verified =
        present(&holder, members, None, |_, _, _| EnvelopeDecision::Skip).expect("verifies");

    assert!(verified.verified.is_empty());
    let skipped: Vec<(usize, &str, Option<&str>)> = verified
        .skipped
        .iter()
        .map(|s: &SkippedEnvelope| (s.index, s.media_type.as_str(), s.kid.as_deref()))
        .collect();
    assert_eq!(
        skipped,
        [
            (0, "application/vc+cose", None),
            (1, "application/vc+jwt", Some(KID))
        ]
    );
}

/// A skip is the caller's word, so an envelope that would have failed is passed and
/// reported, not looked at.
#[test]
fn a_skipped_vc_jwt_is_not_verified_even_when_it_would_fail() {
    let holder = holder();
    let signer = issuer();
    let mut broken = sign_credential(&credential(), &signer, KID, &[]).expect("signs");
    broken.push('A');

    let verified = present(&holder, vec![envelope(&broken)], None, |_, _, _| {
        EnvelopeDecision::Skip
    })
    .expect("verifies");

    assert_eq!(verified.skipped.len(), 1);
}

// ── Verify: every check of a credential, with the same `now` ───────────────────

fn refused_inner(index: usize, source: JoseError) -> Result<(), JoseError> {
    Err(JoseError::CredentialRefused {
        index,
        source: Box::new(source),
    })
}

#[test]
fn a_vc_jwt_signed_by_another_key_refuses_the_presentation() {
    let (holder, issuer) = (holder(), issuer());
    let imposter = issuer_other();
    let members = vec![signed_by(&issuer), signed_by(&imposter)];

    let refused = present(&holder, members, None, |_, _, _| {
        EnvelopeDecision::Verify(issuer.public_key())
    });

    assert_eq!(
        refused.map(|_| ()),
        refused_inner(1, JoseError::SignatureDoesNotVerify)
    );
}

fn issuer_other() -> KeyEntry {
    generated(KeyAlgorithm::Ed25519)
}

#[test]
fn a_key_of_the_wrong_algorithm_refuses_with_the_inner_reason() {
    let (holder, issuer) = (holder(), issuer());
    let wrong = generated(KeyAlgorithm::EcdsaP384).public_key();

    let refused = present(&holder, vec![signed_by(&issuer)], None, |_, _, _| {
        EnvelopeDecision::Verify(wrong.clone())
    });

    assert_eq!(
        refused.map(|_| ()),
        refused_inner(
            0,
            JoseError::AlgorithmMismatch {
                header_alg: Some("EdDSA".to_owned()),
                key_alg: KeyAlgorithm::EcdsaP384,
            }
        )
    );
}

/// The inner token is held to the credential's checks, not only to its signature.
#[test]
fn the_inner_token_goes_through_every_credential_check() {
    let (holder, issuer) = (holder(), issuer());
    let cases: [(Value, Value, JoseError); 3] = [
        // A credential token typed as a presentation.
        (
            json!({"alg": "EdDSA", "kid": KID, "typ": "vp+jwt", "cty": "vp"}),
            credential(),
            JoseError::WrongMediaType {
                member: "typ",
                found: Some("\"vp+jwt\"".to_owned()),
                expected: "vc+jwt",
            },
        ),
        // An `iss` that is not the issuer.
        (
            credential_header(&issuer),
            {
                let mut c = credential();
                c["iss"] = json!("https://attacker.example");
                c
            },
            JoseError::IssuerMismatch {
                iss: "https://attacker.example".to_owned(),
                issuer: Some("https://issuer.example".to_owned()),
            },
        ),
        // A `vc` claim.
        (
            credential_header(&issuer),
            {
                let mut c = credential();
                c["vc"] = json!({});
                c
            },
            JoseError::ReservedClaim { name: "vc" },
        ),
    ];
    for (header, payload, reason) in cases {
        let inner = forge(&issuer, &header, &payload);

        let refused = present(&holder, vec![envelope(&inner)], None, |_, _, _| {
            EnvelopeDecision::Verify(issuer.public_key())
        });

        assert_eq!(refused.map(|_| ()), refused_inner(0, reason));
    }
}

#[test]
fn an_inner_token_that_cannot_be_checked_at_all_refuses_with_its_reason() {
    let (holder, issuer) = (holder(), issuer());
    let token = sign_credential(&credential(), &issuer, KID, &[]).expect("signs");
    let (signing_input, _) = token.rsplit_once('.').expect("segments");

    let refused = present(
        &holder,
        vec![envelope(&format!("{signing_input}.AA"))],
        None,
        |_, _, _| EnvelopeDecision::Verify(issuer.public_key()),
    );

    assert!(matches!(
        refused,
        Err(JoseError::CredentialRefused { index: 0, ref source })
            if matches!(**source, JoseError::Unverifiable { .. })
    ));
}

#[test]
fn verified_and_skipped_envelopes_are_both_reported_in_one_presentation() {
    let (holder, issuer) = (holder(), issuer());
    let members = vec![
        signed_by(&issuer),
        foreign_envelope("application/vc+sd-jwt"),
        signed_by(&issuer),
    ];

    let verified = present(&holder, members, None, |_, media, _| {
        if media == "application/vc+jwt" {
            EnvelopeDecision::Verify(issuer.public_key())
        } else {
            EnvelopeDecision::Skip
        }
    })
    .expect("verifies");

    let positions: Vec<usize> = verified.verified.iter().map(|v| v.index).collect();
    assert_eq!(positions, [0, 2]);
    assert_eq!(verified.skipped.len(), 1);
    assert_eq!(verified.skipped[0].index, 1);
}
