//! The COSE inputs of the W3C VC-JOSE-COSE test suite (rows 27 to 35 of its test
//! mapping), at the layer this module is: bytes in, a `COSE_Sign1` verified or
//! refused. Whether a credential or presentation inside is acceptable, and what
//! `typ` and the content type must be, is for the layer above.
//!
//! The inputs are base64 text in the standard alphabet without padding. Every one is a tagged `COSE_Sign1` (tag 18) whose
//! protected header is a map of `alg` (1), content type (3), `kid` (4) and `typ` (16),
//! in that order, with an empty unprotected header. The `kid` is the key's RFC 7638
//! thumbprint as text.

use super::error::CoseError;
use super::sign1::{ContentType, sign_sign1, verify_sign1};
use super::test_support::{input, key_pair, public_key};

fn text(value: &str) -> Option<ContentType> {
    Some(ContentType::Text(value.to_owned()))
}

/// Row 29: a credential under P-256. `alg` -7, `typ` `application/vc+cose`, content
/// type `application/vc`.
#[test]
fn row_29_the_credential_verifies_under_its_p256_key() {
    let key = public_key("p256");
    let verified = verify_sign1(&input("credential-cose-minimal.txt"), &key).expect("row 29");
    assert_eq!(verified.typ, text("application/vc+cose"));
    assert_eq!(verified.content_type, text("application/vc"));
    assert_eq!(verified.kid, Some(key.thumbprint().into_bytes()));
    assert_eq!(verified.payload.len(), 0x01f2);
    assert!(verified.payload.starts_with(br#"{"@context":["#));
}

/// Row 31: a presentation under P-384. `alg` -35, `typ` `application/vp+cose`.
#[test]
fn row_31_the_presentation_verifies_under_its_p384_key() {
    let key = public_key("p384");
    let verified = verify_sign1(&input("presentation-cose-single.txt"), &key).expect("row 31");
    assert_eq!(verified.typ, text("application/vp+cose"));
    assert_eq!(verified.content_type, text("application/vp"));
    assert_eq!(verified.kid, Some(key.thumbprint().into_bytes()));
    assert!(verified.payload.starts_with(br#"{"@context":["#));
}

/// Row 32: a credential under Ed25519 whose signature is wrong. It decodes, carries
/// `alg` -8 (EdDSA, RFC 9053), and fails on its signature, which is the last thing
/// checked: every structural rule has passed.
#[test]
fn row_32_the_ed25519_credential_fails_on_its_signature() {
    let key = public_key("ed25519");
    assert_eq!(
        verify_sign1(&input("credential-cose-bad-signature.txt"), &key),
        Err(CoseError::Signature)
    );
    // The `alg` is -8: the header is the same bytes with the credential's `alg`, and a
    // key of another algorithm refuses it for its `alg`, which is checked first.
    assert_eq!(
        verify_sign1(
            &input("credential-cose-bad-signature.txt"),
            &public_key("p256")
        ),
        Err(CoseError::AlgorithmMismatch)
    );
    let protected_alg = &input("credential-cose-bad-signature.txt")[..9];
    assert_eq!(
        protected_alg,
        [0xd2, 0x84, 0x58, 0x56, 0xa4, 0x01, 0x27, 0x03, 0x6e]
    );
}

/// Row 30: the credential of row 29 written in hex. It is not base64 COSE.
///
/// Its characters are all in the base64 alphabet, so a base64 decoder does not refuse
/// it. What it decodes to is not a `COSE_Sign1`, and the same bytes written as hex are
/// the bytes of row 29.
#[test]
fn row_30_hex_is_not_base64_cose() {
    use base64::Engine;
    let hex_text = super::test_support::file("credential-cose-minimal-not-base64.txt").trim_end();
    let key = public_key("p256");

    // The hex is exactly row 29's message, so the row differs from row 29 in encoding only.
    assert_eq!(
        hex::decode(hex_text).expect("hex"),
        input("credential-cose-minimal.txt")
    );

    // Every character of the hex is in the base64 alphabet and the text is a whole
    // number of quanta, so a base64 decoder accepts it and reads 987 bytes of nonsense.
    // They begin `77 6f 38`, a text string, where a `COSE_Sign1` begins with a tag or an
    // array: the failure of this row is at the CBOR, not at the base64.
    let decoded = base64::engine::general_purpose::STANDARD_NO_PAD
        .decode(hex_text)
        .expect("the hex happens to be base64");
    assert_eq!(decoded.len(), 987);
    assert_eq!(decoded[..3], [0x77, 0x6f, 0x38]);
    assert!(matches!(
        verify_sign1(&decoded, &key),
        Err(CoseError::Malformed(_))
    ));
    // Read as bytes, the text is not a COSE_Sign1 message.
    assert!(verify_sign1(hex_text.as_bytes(), &key).is_err());
}

/// Row 33: a credential whose `typ` is `really/bad`. At this layer it verifies, and
/// `typ` and the content type come back as written for the caller to judge. That
/// the suite expects a failure for it is the layer above's, and the suite's own
/// issue 24 questions the row: RFC 9596 and VC-JOSE-COSE §3.3.1 say `typ` SHOULD be
/// `application/vc+cose`.
#[test]
fn row_33_a_bad_media_type_is_returned_not_judged() {
    let verified = verify_sign1(
        &input("credential-cose-bad-media-type.txt"),
        &public_key("p384"),
    )
    .expect("the signature is good");
    assert_eq!(verified.typ, text("really/bad"));
    assert_eq!(verified.content_type, text("very/bad"));
}

/// Row 34: a presentation whose `typ` is `bad/type`. The suite maps it to the P-384
/// key, but the input is signed by the P-256 key: its `alg` is -7 and its `kid` is
/// that key's thumbprint. Under the mapped key it is refused for its `alg`, before any
/// signature is checked, so the row fails whatever its `typ` says. Under its own key it
/// verifies, and `typ` comes back as written, as in row 33.
#[test]
fn row_34_is_signed_by_the_p256_key_the_suite_maps_to_p384() {
    let input = input("presentation-cose-bad-media-type.txt");
    assert_eq!(
        verify_sign1(&input, &public_key("p384")),
        Err(CoseError::AlgorithmMismatch)
    );
    let key = public_key("p256");
    let verified = verify_sign1(&input, &key).expect("the signature is good");
    assert_eq!(verified.typ, text("bad/type"));
    assert_eq!(verified.content_type, text("very/bad"));
    assert_eq!(verified.kid, Some(key.thumbprint().into_bytes()));
}

/// Row 35: a presentation, under P-256, whose enveloped credential is invalid. The
/// presentation's own signature is good, and the failure is in what it carries.
#[test]
fn row_35_the_presentation_itself_verifies() {
    let verified = verify_sign1(
        &input("presentation-cose-bad-credential.txt"),
        &public_key("p256"),
    )
    .expect("the presentation's signature is good");
    assert_eq!(verified.typ, text("application/vp+cose"));
}

/// A key of another algorithm never verifies a suite input, whatever its `alg` says.
#[test]
fn a_suite_input_does_not_verify_under_another_algorithms_key() {
    let row_29 = input("credential-cose-minimal.txt");
    for other in ["ed25519", "p384", "p521"] {
        assert_eq!(
            verify_sign1(&row_29, &public_key(other)),
            Err(CoseError::AlgorithmMismatch),
            "{other}"
        );
    }
}

/// Signing the payload of row 29 under the suite's P-256 key with its `kid`, `typ` and
/// content type writes the suite's message but for the signature: the same tag,
/// array, protected header, unprotected header and payload.
#[test]
fn signing_writes_the_message_the_suite_writes_but_for_its_signature() {
    let suite = input("credential-cose-minimal.txt");
    let verified = verify_sign1(&suite, &public_key("p256")).unwrap();
    let ours = sign_sign1(
        &key_pair("p256"),
        &verified.payload,
        verified.kid.as_deref().unwrap(),
        Some("application/vc+cose"),
        Some("application/vc"),
    )
    .unwrap();
    // The signature is a 64-byte string with its two-byte header at the end.
    let unsigned = suite.len() - 66;
    assert_eq!(ours.len(), suite.len());
    assert_eq!(ours[..unsigned], suite[..unsigned]);
    assert_eq!(verify_sign1(&ours, &public_key("p256")).unwrap(), verified);
}
