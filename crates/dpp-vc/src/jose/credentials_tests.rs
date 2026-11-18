//! The credentials a presentation envelops: what `verify_presentation` lists without
//! verifying them, and what `verify_credentials` and `UnverifiedEnvelope::verify` refuse.
//!
//! `verify_presentation` never verifies an envelope. Each test here gives a credential
//! its issuer's key through `verify_credentials`, or verifies one with `verify`.

use serde_json::{Value, json};

use dpp_crypto::jws::KeyAlgorithm;
use dpp_crypto::keystore::KeyEntry;

use super::{
    JoseError, VerifiedPresentation, sign_credential, sign_presentation, verify_presentation,
};
use crate::test_support::{
    KID, credential, credential_header, envelope, foreign_envelope, forge, generated, presentation,
};

fn holder() -> KeyEntry {
    generated(KeyAlgorithm::EcdsaP256)
}

fn issuer() -> KeyEntry {
    generated(KeyAlgorithm::Ed25519)
}

/// A presentation of `members`, signed by `holder` and verified. Its envelopes are
/// listed, and none is verified.
fn present(holder: &KeyEntry, members: Vec<Value>) -> Result<VerifiedPresentation, JoseError> {
    let token = sign_presentation(&presentation(members), holder, KID, &[]).expect("signs");
    verify_presentation(&token, &holder.public_key(), None)
}

/// The envelope of a credential signed by `issuer`.
fn signed_by(issuer: &KeyEntry) -> Value {
    envelope(&sign_credential(&credential(), issuer, KID, &[]).expect("signs"))
}

/// The envelope of a credential signed by `issuer` with the `id` given.
fn signed_with_id(issuer: &KeyEntry, id: &str) -> Value {
    let mut document = credential();
    document["id"] = json!(id);
    envelope(&sign_credential(&document, issuer, KID, &[]).expect("signs"))
}

fn refused_inner(index: usize, source: JoseError) -> Result<(), JoseError> {
    Err(JoseError::CredentialRefused {
        index,
        source: Box::new(source),
    })
}

fn issuer_other() -> KeyEntry {
    generated(KeyAlgorithm::Ed25519)
}

#[test]
fn an_envelope_with_no_key_refuses_the_credentials_and_names_it() {
    let (holder, issuer) = (holder(), issuer());
    let verified =
        present(&holder, vec![signed_by(&issuer), signed_by(&issuer)]).expect("verifies");

    let refused = verified.verify_credentials(None, |envelope| {
        (envelope.index == 0).then(|| issuer.public_key())
    });

    assert_eq!(
        refused,
        Err(JoseError::CredentialKeyUnknown {
            index: 1,
            kid: Some(KID.to_owned()),
        })
    );
}

#[test]
fn an_envelope_of_another_format_cannot_be_verified_here() {
    let holder = holder();
    let key = issuer().public_key();

    for media_type in ["application/vc+sd-jwt", "application/vc+cose", "text/plain"] {
        let verified = present(&holder, vec![foreign_envelope(media_type)]).expect("verifies");
        let unsupported = JoseError::UnsupportedEnvelopeFormat {
            index: 0,
            media_type: media_type.to_owned(),
        };

        assert_eq!(
            verified.envelopes[0].verify(&key, None).map(|_| ()),
            Err(unsupported.clone()),
            "{media_type}"
        );
        assert_eq!(
            verified
                .verify_credentials(None, |_| Some(key.clone()))
                .map(|_| ()),
            Err(unsupported),
            "{media_type}"
        );
    }
}

#[test]
fn every_envelope_is_listed_with_its_index_media_type_and_kid() {
    let (holder, issuer) = (holder(), issuer());
    let verified = present(
        &holder,
        vec![foreign_envelope("application/vc+cose"), signed_by(&issuer)],
    )
    .expect("verifies");

    let listed: Vec<(usize, &str, Option<&str>)> = verified
        .envelopes
        .iter()
        .map(|e| (e.index, e.media_type.as_str(), e.kid.as_deref()))
        .collect();
    assert_eq!(
        listed,
        [
            (0, "application/vc+cose", None),
            (1, "application/vc+jwt", Some(KID))
        ]
    );
}

/// `verify_presentation` checks the shape of an envelope and nothing inside it, so a
/// credential that would fail is listed, and fails only when it is verified.
#[test]
fn verify_presentation_does_not_verify_an_envelope_that_would_fail() {
    let holder = holder();
    let signer = issuer();
    let mut broken = sign_credential(&credential(), &signer, KID, &[]).expect("signs");
    broken.push('A');

    let verified =
        present(&holder, vec![envelope(&broken)]).expect("the envelope's shape is right");

    assert!(matches!(
        verified.envelopes[0].verify(&signer.public_key(), None),
        Err(JoseError::CredentialRefused { index: 0, .. })
    ));
}

#[test]
fn a_vc_jwt_signed_by_another_key_refuses_the_presentation() {
    let (holder, issuer) = (holder(), issuer());
    let imposter = issuer_other();
    let verified =
        present(&holder, vec![signed_by(&issuer), signed_by(&imposter)]).expect("verifies");

    let refused = verified.verify_credentials(None, |_| Some(issuer.public_key()));

    assert_eq!(
        refused.map(|_| ()),
        refused_inner(1, JoseError::SignatureDoesNotVerify)
    );
}

#[test]
fn a_key_of_the_wrong_algorithm_refuses_with_the_inner_reason() {
    let (holder, issuer) = (holder(), issuer());
    let wrong = generated(KeyAlgorithm::EcdsaP384).public_key();
    let verified = present(&holder, vec![signed_by(&issuer)]).expect("verifies");

    let refused = verified.verify_credentials(None, |_| Some(wrong.clone()));

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
        let verified =
            present(&holder, vec![envelope(&inner)]).expect("the envelope's shape is right");

        let refused = verified.verify_credentials(None, |_| Some(issuer.public_key()));

        assert_eq!(refused.map(|_| ()), refused_inner(0, reason));
    }
}

#[test]
fn an_inner_token_that_cannot_be_checked_at_all_refuses_with_its_reason() {
    let (holder, issuer) = (holder(), issuer());
    let token = sign_credential(&credential(), &issuer, KID, &[]).expect("signs");
    let (signing_input, _) = token.rsplit_once('.').expect("segments");
    let verified = present(&holder, vec![envelope(&format!("{signing_input}.AA"))])
        .expect("the envelope's shape is right");

    let refused = verified.verify_credentials(None, |_| Some(issuer.public_key()));

    assert!(matches!(
        refused,
        Err(JoseError::CredentialRefused { index: 0, ref source })
            if matches!(**source, JoseError::Unverifiable { .. })
    ));
}

#[test]
fn every_envelope_of_a_mixed_presentation_is_listed_in_order() {
    let (holder, issuer) = (holder(), issuer());
    let verified = present(
        &holder,
        vec![
            signed_by(&issuer),
            foreign_envelope("application/vc+sd-jwt"),
            signed_by(&issuer),
        ],
    )
    .expect("verifies");

    let listed: Vec<(usize, &str)> = verified
        .envelopes
        .iter()
        .map(|e| (e.index, e.media_type.as_str()))
        .collect();
    assert_eq!(
        listed,
        [
            (0, "application/vc+jwt"),
            (1, "application/vc+sd-jwt"),
            (2, "application/vc+jwt"),
        ]
    );
    assert!(
        verified.envelopes[0]
            .verify(&issuer.public_key(), None)
            .is_ok()
    );
    assert!(
        verified.envelopes[2]
            .verify(&issuer.public_key(), None)
            .is_ok()
    );
}

#[test]
fn verify_credentials_returns_every_payload_in_envelope_order() {
    let (holder, issuer) = (holder(), issuer());
    let verified = present(
        &holder,
        vec![
            signed_with_id(&issuer, "urn:example:first"),
            signed_with_id(&issuer, "urn:example:second"),
        ],
    )
    .expect("verifies");
    let mut asked = Vec::new();

    let payloads = verified
        .verify_credentials(None, |envelope| {
            asked.push(envelope.index);
            Some(issuer.public_key())
        })
        .expect("every credential verifies");

    assert_eq!(asked, [0, 1]);
    assert_eq!(payloads.len(), 2);
    assert_eq!(payloads[0]["id"], "urn:example:first");
    assert_eq!(payloads[1]["id"], "urn:example:second");
}
