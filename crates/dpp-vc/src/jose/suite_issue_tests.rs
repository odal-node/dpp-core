//! The W3C VC-JOSE-COSE test suite, rows 1 to 5: the issuance inputs, signed with the
//! suite's own keys and verified back.
//!
//! Each key is built from the `secretKeyJwk` of its key file with
//! `KeyEntry::from_private_jwk`, and each token is signed with the `kid` the key
//! file's `id` gives: an absolute URL, which VC-JOSE-COSE §4.2 asks for when the
//! credential has no `iss`. It is verified under the `publicKeyJwk` of the same
//! file. Rows 3 and 5 are the two the brief singled out, and each is explained where
//! it is tested.

use super::VerifyOptions;
use super::{
    JoseError, sign_credential, sign_presentation, verify_credential, verify_presentation,
};
use crate::test_support::NOT_REQUIRED;
use crate::test_support::{json, key_id, public_key, signing_key};

#[test]
fn row_1_a_basic_credential_is_issued_and_verifies_back() {
    let credential = json("credential-minimal.json");

    let token =
        sign_credential(&credential, &signing_key("p256"), &key_id("p256"), &[]).expect("issued");

    let verified =
        verify_credential(&token, &public_key("p256"), VerifyOptions::new()).expect("verifies");
    assert_eq!(verified, credential);
}

#[test]
fn row_2_a_credential_with_every_optional_field_is_issued_and_verifies_back() {
    let credential = json("credential-full.json");

    let token =
        sign_credential(&credential, &signing_key("p384"), &key_id("p384"), &[]).expect("issued");

    let verified =
        verify_credential(&token, &public_key("p384"), VerifyOptions::new()).expect("verifies");
    assert_eq!(verified, credential);
}

/// Row 3's presentation embeds a `vc+jwt` envelope whose protected-header segment is
/// 89 characters, which is not a base64url length: it begins `yJraWQ` where a JSON
/// object's encoding begins `eyJraWQ`. Issuing succeeds, since envelopes are not
/// parsed at issuance, as the suite expects. Verifying the result is refused, as a
/// malformed envelope, because the envelope is not a compact JWS.
#[test]
fn row_3_a_presentation_with_a_malformed_envelope_is_issued_and_then_refused() {
    let presentation = json("presentation-single.json");

    let token = sign_presentation(&presentation, &signing_key("p256"), &key_id("p256"), &[])
        .expect("issued: envelopes are not parsed at issuance");

    assert_eq!(
        verify_presentation(&token, &public_key("p256"), VerifyOptions::new()).map(|_| ()),
        Err(JoseError::MalformedEnvelope {
            index: 0,
            reason: "the vc+jwt data is not a compact JWS".to_owned(),
        })
    );
}

/// Row 4's presentation holds a `vc+jwt` signed under P-256 and a `vc+sd-jwt` and a
/// `vc+cose`. The presentation verifies under its own key and lists the three
/// envelopes. Only the `vc+jwt` and the `vc+sd-jwt` can be verified here, under P-256 and P-384.
#[test]
fn row_4_a_presentation_of_three_formats_is_issued_and_verifies_back() {
    let presentation = json("presentation-multiple.json");

    let token = sign_presentation(&presentation, &signing_key("p521"), &key_id("p521"), &[])
        .expect("issued");

    let verified =
        verify_presentation(&token, &public_key("p521"), VerifyOptions::new()).expect("verifies");
    assert_eq!(verified.payload(), &presentation);
    let listed: Vec<(usize, &str)> = verified
        .envelopes()
        .iter()
        .map(|e| (e.index, e.media_type.as_str()))
        .collect();
    assert_eq!(
        listed,
        [
            (0, "application/vc+jwt"),
            (1, "application/vc+sd-jwt"),
            (2, "application/vc+cose"),
        ]
    );
    assert!(
        verified.envelopes()[0]
            .verify(&public_key("p256"), VerifyOptions::new(), &NOT_REQUIRED)
            .is_ok()
    );
    assert!(
        verified.envelopes()[1]
            .verify(&public_key("p384"), VerifyOptions::new(), &NOT_REQUIRED)
            .is_ok()
    );
    assert!(matches!(
        verified.envelopes()[2].verify(&public_key("p256"), VerifyOptions::new(), &NOT_REQUIRED),
        Err(JoseError::UnsupportedEnvelopeFormat { index: 2, .. })
    ));
}

/// Row 5 carries two members that are in no VC Data Model 2.0 vocabulary, and the
/// suite expects a refusal. The issuer is refused for exactly that, and the same
/// input signs when the caller declares both terms.
#[test]
fn row_5_a_credential_with_undeclared_members_is_refused_at_issuance() {
    let credential = json("credential-unknown-extensions.json");

    let refused = sign_credential(&credential, &signing_key("p521"), &key_id("p521"), &[])
        .expect_err("refused");

    assert_eq!(
        refused,
        JoseError::UndeclaredMembers {
            names: vec!["anotherBadOne".to_owned(), "badExtension".to_owned()]
        }
    );
    let declared = ["badExtension", "anotherBadOne"];
    assert!(
        sign_credential(
            &credential,
            &signing_key("p521"),
            &key_id("p521"),
            &declared
        )
        .is_ok()
    );
}
