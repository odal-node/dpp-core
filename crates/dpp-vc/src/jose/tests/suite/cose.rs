//! The W3C VC-JOSE-COSE test suite, rows 27 to 35, the COSE rows: the two issuance inputs
//! signed with the suite's own keys and verified back, and the seven verification inputs
//! under the key the suite's mapping pairs each with.
//!
//! The harness's mapping is `Ok` for `success` and `Err` for `failure`, and [`harness`] is
//! that mapping. Every row also asserts the specific [`JoseError`], and for a row that is
//! refused for something other than its signature, that the signature is valid, so the refusal
//! is provably the check it names. The inputs are base64 text of the standard alphabet with no
//! padding, as the suite writes them. The layer below, `dpp_crypto::cose`, has its own tests
//! of the same inputs.

use dpp_crypto::cose::{ContentType, CoseError, verify_sign1};
use dpp_crypto::jws::PublicKey;

use super::super::super::{
    JoseError, VerifyOptions, sign_credential_cose, sign_presentation_cose, verify_credential_cose,
    verify_presentation_cose,
};
use crate::test_support::{NOT_REQUIRED, cose_input, json, key_id, public_key, signing_key};

fn harness<T>(result: &Result<T, JoseError>) -> &'static str {
    if result.is_ok() { "success" } else { "failure" }
}

fn signature_is_valid(file: &str, key: &PublicKey) -> bool {
    verify_sign1(&cose_input(file), key).is_ok()
}

fn verify_vc(file: &str, key: &PublicKey) -> Result<serde_json::Value, JoseError> {
    verify_credential_cose(&cose_input(file), key, VerifyOptions::new())
}

fn verify_vp(
    file: &str,
    key: &PublicKey,
) -> Result<super::super::super::VerifiedPresentation, JoseError> {
    verify_presentation_cose(&cose_input(file), key, VerifyOptions::new())
}

/// Row 27: the suite's credential, signed under the key of `vm-did-ed25519.json`, whose `id` is
/// a DID URL. The suite expects success. The message is EdDSA (`alg` -8), and verifies back.
#[test]
fn row_27_a_basic_credential_is_issued_under_ed25519_and_verifies_back() {
    let credential = json("credential-minimal.json");

    let message = sign_credential_cose(
        &credential,
        &signing_key("did-ed25519"),
        &key_id("did-ed25519"),
        &[],
    )
    .expect("issued");

    let verified = verify_sign1(&message, &public_key("did-ed25519")).expect("a COSE_Sign1");
    assert_eq!(
        verified.kid.as_deref(),
        Some(b"did:example:vc-jose-cose#key-4".as_slice())
    );
    assert_eq!(
        verified.typ,
        Some(ContentType::Text("application/vc+cose".to_owned()))
    );
    assert_eq!(
        verify_credential_cose(&message, &public_key("did-ed25519"), VerifyOptions::new()),
        Ok(credential)
    );
}

/// Row 28: the suite's presentation, signed under P-384. Its envelope is not a JWS, and is not
/// opened at issuance, as for row 3.
#[test]
fn row_28_a_basic_presentation_is_issued_under_p384_and_verifies_back() {
    let presentation = json("presentation-single.json");

    let message = sign_presentation_cose(&presentation, &signing_key("p384"), &key_id("p384"), &[])
        .expect("issued");

    let verified = verify_presentation_cose(&message, &public_key("p384"), VerifyOptions::new())
        .expect("verifies");
    assert_eq!(verified.payload(), &presentation);
}

/// Row 29: a credential under P-256, `alg` -7, `typ` `application/vc+cose`, no `iss`, and a
/// `kid` that is the key's RFC 7638 thumbprint, not a URL. It verifies: §4.2 holds a `kid` to a
/// URL only "when using controlled identifier documents", and the key is the caller's.
#[test]
fn row_29_a_basic_credential_verifies_under_its_p256_key() {
    let result = verify_vc("credential-cose-minimal.txt", &public_key("p256"));

    assert_eq!(harness(&result), "success");
    let credential = result.expect("verified");
    assert_eq!(credential["issuer"], "https://example.issuer/vc-jose-cose");
    assert_eq!(credential["type"][1], "ExampleAlumniCredential");
    // No `iss`, and a bare `kid`: the case the `kid` rule used to refuse.
    let verified = verify_sign1(
        &cose_input("credential-cose-minimal.txt"),
        &public_key("p256"),
    )
    .unwrap();
    assert_eq!(verified.cwt_claims, dpp_crypto::cose::CwtClaims::default());
    assert_eq!(
        verified.kid,
        Some(public_key("p256").thumbprint().into_bytes())
    );
}

/// Row 30 is row 29's bytes written as hex. Every character is in the base64 alphabet and the
/// length is a multiple of four, so the base64 reads, and what it reads is 987 bytes that begin
/// with a text string and are not a `COSE_Sign1`. That is why it is refused: no input is tested
/// for being hex, and none should be.
#[test]
fn row_30_hex_is_refused_because_what_it_decodes_to_is_not_a_cose_sign1() {
    let text = crate::test_support::file("credential-cose-minimal-not-base64.txt").trim_end();
    // The same bytes as row 29.
    assert_eq!(
        hex::decode(text).expect("hex"),
        cose_input("credential-cose-minimal.txt")
    );

    let decoded = cose_input("credential-cose-minimal-not-base64.txt");
    assert_eq!(decoded.len(), 987);
    let result = verify_credential_cose(&decoded, &public_key("p256"), VerifyOptions::new());

    assert_eq!(harness(&result), "failure");
    assert!(matches!(
        result,
        Err(JoseError::Cose(CoseError::Malformed(_)))
    ));
}

/// Row 31: a presentation under P-384. Its one envelope is a `vc+jwt` whose data is
/// `<header>.<payload>....`, not a compact JWS. The presentation verifies, since an envelope is
/// judged for what it is and not for what is inside; the credential is refused when it is
/// verified.
#[test]
fn row_31_a_basic_presentation_verifies_and_its_envelope_is_judged_as_an_envelope() {
    let result = verify_vp("presentation-cose-single.txt", &public_key("p384"));

    assert_eq!(harness(&result), "success");
    let presentation = result.expect("verified");
    assert_eq!(presentation.envelopes().len(), 1);
    assert_eq!(presentation.envelopes()[0].media_type, "application/vc+jwt");
    assert_eq!(
        presentation.envelopes()[0].verify(
            &public_key("p256"),
            VerifyOptions::new(),
            &NOT_REQUIRED
        ),
        Err(JoseError::CredentialRefused {
            index: 0,
            source: Box::new(JoseError::NotCompactJws),
        })
    );
}

/// Row 32: a credential under Ed25519 whose signature is wrong. Its structure and `alg` (-8)
/// are good, and the signature is the last thing checked.
#[test]
fn row_32_a_credential_with_a_bad_signature_is_refused_for_its_signature() {
    let result = verify_vc("credential-cose-bad-signature.txt", &public_key("ed25519"));

    assert_eq!(harness(&result), "failure");
    assert_eq!(result, Err(JoseError::Cose(CoseError::Signature)));
}

/// Row 33: a credential whose `typ` is `really/bad` and whose content type is `very/bad`. Its
/// signature is good under P-384, and `typ` and the content type are held to the policy of the
/// `vc+jwt` form, so it is refused for `typ`. The suite's own issue 24 questions this row:
/// §3.3.1 says `typ` SHOULD be `application/vc+cose`, and this verifier reads it as it reads
/// the `vc+jwt` form's, with `typ` required and exact.
#[test]
fn row_33_a_credential_with_a_bad_media_type_is_refused_for_its_typ() {
    let file = "credential-cose-bad-media-type.txt";
    let key = public_key("p384");

    assert!(signature_is_valid(file, &key));
    let result = verify_vc(file, &key);

    assert_eq!(harness(&result), "failure");
    assert_eq!(
        result,
        Err(JoseError::WrongMediaType {
            member: "typ",
            found: Some("\"really/bad\"".to_owned()),
            expected: "vc+cose",
        })
    );
}

/// Row 34: a presentation whose `typ` is `bad/type`. The suite maps it to the P-384 key, but
/// the message is signed by the P-256 key, and under the mapped key it is refused for its
/// algorithm before any signature is checked: it fails, for a reason that is not its `typ`.
/// Under its own key its signature is good and it is refused for `typ`.
#[test]
fn row_34_is_refused_for_its_algorithm_under_the_mapped_key_and_for_its_typ_under_its_own() {
    let file = "presentation-cose-bad-media-type.txt";

    let mapped = verify_vp(file, &public_key("p384"));
    assert_eq!(harness(&mapped), "failure");
    assert_eq!(
        mapped.map(|_| ()),
        Err(JoseError::Cose(CoseError::AlgorithmMismatch))
    );

    assert!(signature_is_valid(file, &public_key("p256")));
    assert_eq!(
        verify_vp(file, &public_key("p256")).map(|_| ()),
        Err(JoseError::WrongMediaType {
            member: "typ",
            found: Some("\"bad/type\"".to_owned()),
            expected: "vp+cose",
        })
    );
}

/// Row 35: a presentation under P-256 whose one envelope has the `id` `bad-credential`, which is
/// not a `data:` URL. The presentation's signature is good, and it is refused for the envelope,
/// without any credential being verified.
#[test]
fn row_35_a_presentation_with_an_envelope_that_is_not_one_is_refused_for_the_envelope() {
    let file = "presentation-cose-bad-credential.txt";
    let key = public_key("p256");

    assert!(signature_is_valid(file, &key));
    let result = verify_vp(file, &key);

    assert_eq!(harness(&result), "failure");
    assert_eq!(
        result.map(|_| ()),
        Err(JoseError::MalformedEnvelope {
            index: 0,
            reason: "id is not a data: URL".to_owned(),
        })
    );
}

/// The suite's valid inputs verify under no other key: the key decides the algorithm.
#[test]
fn a_suite_input_does_not_verify_under_another_algorithms_key() {
    for other in ["ed25519", "p384", "p521"] {
        assert_eq!(
            verify_vc("credential-cose-minimal.txt", &public_key(other)),
            Err(JoseError::Cose(CoseError::AlgorithmMismatch)),
            "{other}"
        );
    }
}
