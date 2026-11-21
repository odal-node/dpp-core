//! The CWT claims of a COSE message, applied as the JOSE form applies the same JWT claims
//! (VC-JOSE-COSE §3.3.3; RFC 9597; RFC 8392).
//!
//! Messages a signer would never write are forged with a genuine signature
//! ([`super::cose_support`]), so a refusal is the rule it names and not a signature that never
//! verified.

use serde_json::json;

use super::cose_support::{
    AUDIENCE, Header, array, claim, cred_header, cwt, forge_document, forge_with_unprotected,
    forged_credential, holder, issuer, map, nint, tstr, uint, verify_vc, verify_vc_with,
};
use super::{JoseError, VerifyOptions, verify_presentation_cose};
use crate::test_support::{KID, credential, presentation, public_key};
use dpp_crypto::cose::CoseError;

// ── CWT claims (§3.3.3, RFC 9597, RFC 8392) ──────────────────────────────────

#[test]
fn an_iss_claim_must_equal_the_issuer() {
    let wrong = forged_credential(
        &credential(),
        &cwt(vec![claim(1, tstr("https://other.example"))]),
    );
    let right = forged_credential(
        &credential(),
        &cwt(vec![claim(1, tstr("https://issuer.example"))]),
    );

    assert_eq!(
        verify_vc(&wrong),
        Err(JoseError::IssuerMismatch {
            iss: "https://other.example".to_owned(),
            issuer: Some("https://issuer.example".to_owned()),
        })
    );
    assert!(verify_vc(&right).is_ok());
}

/// §4.1.2: when the issuer is an object, `iss` must match its `id`.
#[test]
fn an_iss_claim_is_compared_with_the_id_of_an_issuer_object() {
    let mut document = credential();
    document["issuer"] = json!({"id": "https://issuer.example", "name": "Example"});
    let right = forged_credential(
        &document,
        &cwt(vec![claim(1, tstr("https://issuer.example"))]),
    );
    let wrong = forged_credential(
        &document,
        &cwt(vec![claim(1, tstr("https://other.example"))]),
    );

    assert!(verify_vc(&right).is_ok());
    assert!(matches!(
        verify_vc(&wrong),
        Err(JoseError::IssuerMismatch { .. })
    ));
}

#[test]
fn an_expired_credential_is_refused_under_a_now_and_not_without_one() {
    let message = forged_credential(&credential(), &cwt(vec![claim(4, uint(1_000))]));

    assert_eq!(
        verify_vc_with(&message, VerifyOptions::new().at(1_000)),
        Err(JoseError::Expired)
    );
    assert_eq!(
        verify_vc_with(&message, VerifyOptions::new().at(5_000)),
        Err(JoseError::Expired)
    );
    // The control: one second earlier, and no `now`, verify.
    assert!(verify_vc_with(&message, VerifyOptions::new().at(999)).is_ok());
    assert!(verify_vc(&message).is_ok());
}

#[test]
fn a_credential_that_is_not_yet_valid_is_refused_under_a_now_and_not_without_one() {
    let message = forged_credential(&credential(), &cwt(vec![claim(5, uint(2_000))]));

    assert_eq!(
        verify_vc_with(&message, VerifyOptions::new().at(1_999)),
        Err(JoseError::NotYetValid)
    );
    assert!(verify_vc_with(&message, VerifyOptions::new().at(2_000)).is_ok());
    assert!(verify_vc(&message).is_ok());
}

/// A NumericDate may be fractional, and `iat` is never checked (§3.3.3: it describes the
/// signature, not the credential).
#[test]
fn a_fractional_exp_is_compared_exactly_and_iat_is_not_checked() {
    let float = |x: f64| [vec![0xfb], x.to_be_bytes().to_vec()].concat();
    let message = forged_credential(
        &credential(),
        &cwt(vec![
            claim(4, float(1_000.5)),
            claim(6, uint(4_000_000_000)),
        ]),
    );

    assert!(verify_vc_with(&message, VerifyOptions::new().at(1_000)).is_ok());
    assert_eq!(
        verify_vc_with(&message, VerifyOptions::new().at(1_001)),
        Err(JoseError::Expired)
    );
}

#[test]
fn an_audience_claim_must_name_the_audience_the_verifier_identifies_as() {
    let named = |value: Vec<u8>| forged_credential(&credential(), &cwt(vec![claim(3, value)]));
    let one = named(tstr(AUDIENCE));
    let several = named(array(&[tstr("https://a.example"), tstr(AUDIENCE)]));
    let elsewhere = named(tstr("https://other.example"));
    let nobody = named(array(&[]));
    let for_audience = VerifyOptions::new().for_audience(AUDIENCE);

    // The controls: it names the audience.
    assert!(verify_vc_with(&one, for_audience).is_ok());
    assert!(verify_vc_with(&several, for_audience).is_ok());
    // Another audience.
    assert_eq!(
        verify_vc_with(&elsewhere, for_audience),
        Err(JoseError::AudienceMismatch {
            audience: Some(AUDIENCE.to_owned())
        })
    );
    assert!(matches!(
        verify_vc_with(&nobody, for_audience),
        Err(JoseError::AudienceMismatch { .. })
    ));
    // No audience given: a token that carries `aud` is refused (RFC 7519 §4.1.3).
    for message in [&one, &several, &elsewhere] {
        assert_eq!(
            verify_vc(message),
            Err(JoseError::AudienceMismatch { audience: None })
        );
    }
    // A token with no `aud` passes with or without an audience.
    let none = forged_credential(&credential(), &cred_header());
    assert!(verify_vc(&none).is_ok());
    assert!(verify_vc_with(&none, for_audience).is_ok());
}

/// The same rules hold for a presentation: its own message's claims, not its envelopes'.
#[test]
fn a_presentations_claims_are_applied_to_the_presentation() {
    let holder_key = holder();
    let header = Header::presentation(KID);
    let verify = |header: &Header, options| {
        verify_presentation_cose(
            &forge_document(&holder_key, header, &presentation(vec![])),
            &public_key("p384"),
            options,
        )
        .map(|_| ())
    };

    assert_eq!(
        verify(
            &header.clone().cwt(vec![claim(4, uint(10))]),
            VerifyOptions::new().at(10)
        ),
        Err(JoseError::Expired)
    );
    assert_eq!(
        verify(
            &header.clone().cwt(vec![claim(5, uint(10))]),
            VerifyOptions::new().at(9)
        ),
        Err(JoseError::NotYetValid)
    );
    assert_eq!(
        verify(
            &header
                .clone()
                .cwt(vec![claim(3, tstr("https://other.example"))]),
            VerifyOptions::new().for_audience(AUDIENCE)
        ),
        Err(JoseError::AudienceMismatch {
            audience: Some(AUDIENCE.to_owned())
        })
    );
    // As for a `vp+jwt`, `iss` is not compared with the holder.
    assert_eq!(
        verify(
            &header.cwt(vec![claim(1, tstr("https://anyone.example"))]),
            VerifyOptions::new()
        ),
        Ok(())
    );
}

/// RFC 9597 §1: the CWT Claims header parameter is not used beyond the protection it has, and
/// an unprotected one has none.
#[test]
fn claims_in_the_unprotected_header_are_refused() {
    let claims = map(&[claim(1, tstr("https://issuer.example"))]);
    let message = forge_with_unprotected(
        &issuer(),
        &cred_header(),
        &map(&[(uint(15), claims.clone())]),
        &serde_json::to_vec(&credential()).unwrap(),
    );
    // The control: the same claims, protected.
    let protected = forged_credential(
        &credential(),
        &cwt(vec![claim(1, tstr("https://issuer.example"))]),
    );

    assert_eq!(
        verify_vc(&message),
        Err(JoseError::Cose(CoseError::CwtClaimsNotProtected))
    );
    assert!(verify_vc(&protected).is_ok());
}

#[test]
fn a_claim_of_the_wrong_type_or_a_repeated_key_is_refused() {
    for pairs in [
        vec![claim(1, uint(5))],
        vec![claim(4, tstr("tomorrow"))],
        vec![claim(1, tstr("a")), claim(1, tstr("a"))],
    ] {
        let message = forged_credential(&credential(), &cwt(pairs.clone()));

        assert!(
            matches!(
                verify_vc(&message),
                Err(JoseError::Cose(CoseError::Malformed(_)))
            ),
            "{pairs:?}"
        );
    }
}

#[test]
fn claims_that_are_not_understood_are_ignored() {
    let message = forged_credential(
        &credential(),
        &cwt(vec![
            claim(7, tstr("cti")),
            (nint(-5), tstr("private")),
            (tstr("x"), uint(1)),
        ]),
    );

    assert!(verify_vc(&message).is_ok());
}
