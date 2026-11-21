//! The CWT Claims header parameter (label 15, RFC 9597 §2): read from the protected
//! header, refused in the unprotected one, held to the types of RFC 8392 §4, and the
//! claims this crate does not read ignored.

use super::error::CoseError;
use super::test_support::{
    array, bstr, map, message, nint, no_unprotected, ok, protected_with, signer, tstr, uint, verify,
};

/// A claim: its encoded key and encoded value.
type Pair = (Vec<u8>, Vec<u8>);

/// The CWT Claims header parameter holding `claims`, as a protected-header pair.
fn cwt(claims: &[(Vec<u8>, Vec<u8>)]) -> (Vec<u8>, Vec<u8>) {
    (uint(15), map(claims))
}

fn protected_claims(claims: &[(Vec<u8>, Vec<u8>)]) -> Vec<u8> {
    protected_with(&[cwt(claims)])
}

#[test]
fn a_message_with_no_cwt_claims_returns_none_of_them() {
    let verified = ok(&protected_with(&[]), &no_unprotected()).expect("verifies");
    assert_eq!(verified.cwt_claims, super::CwtClaims::default());
}

#[test]
fn the_registered_claims_of_the_protected_header_are_returned() {
    let protected = protected_claims(&[
        (uint(1), tstr("https://issuer.example")),
        (uint(2), tstr("subject")),
        (uint(3), tstr("https://verifier.example")),
        (uint(4), uint(2_000_000_000)),
        (uint(5), uint(1_000_000_000)),
        (uint(6), nint(-5)),
    ]);
    let claims = ok(&protected, &no_unprotected())
        .expect("verifies")
        .cwt_claims;
    assert_eq!(claims.iss.as_deref(), Some("https://issuer.example"));
    assert_eq!(claims.sub.as_deref(), Some("subject"));
    assert_eq!(
        claims.aud,
        Some(vec!["https://verifier.example".to_owned()])
    );
    assert_eq!(claims.exp, Some(2_000_000_000.0));
    assert_eq!(claims.nbf, Some(1_000_000_000.0));
    assert_eq!(claims.iat, Some(-5.0));
}

#[test]
fn an_audience_array_is_returned_entry_by_entry() {
    let protected = protected_claims(&[(uint(3), array(&[tstr("a"), tstr("b")]))]);
    let claims = ok(&protected, &no_unprotected())
        .expect("verifies")
        .cwt_claims;
    assert_eq!(claims.aud, Some(vec!["a".to_owned(), "b".to_owned()]));
}

#[test]
fn a_numeric_date_may_be_a_float_of_any_width() {
    // 1.5 as binary16 (0xf9 3e00), as binary32 (0xfa 3fc00000) and as binary64.
    for (key, item) in [
        (4_u64, vec![0xf9, 0x3e, 0x00]),
        (5, vec![0xfa, 0x3f, 0xc0, 0x00, 0x00]),
        (6, [vec![0xfb], 1.5_f64.to_be_bytes().to_vec()].concat()),
    ] {
        let claims = ok(&protected_claims(&[(uint(key), item)]), &no_unprotected())
            .expect("verifies")
            .cwt_claims;
        let seen = [claims.exp, claims.nbf, claims.iat][(key - 4) as usize];
        assert_eq!(seen, Some(1.5), "key {key}");
    }
}

#[test]
fn a_numeric_date_that_is_not_finite_is_refused() {
    // binary16 infinity, binary32 NaN, binary64 negative infinity.
    for item in [
        vec![0xf9, 0x7c, 0x00],
        vec![0xfa, 0x7f, 0xc0, 0x00, 0x00],
        [vec![0xfb], f64::NEG_INFINITY.to_be_bytes().to_vec()].concat(),
    ] {
        assert!(matches!(
            ok(&protected_claims(&[(uint(4), item)]), &no_unprotected()),
            Err(CoseError::Malformed(_))
        ));
    }
}

/// RFC 9597 §1: nothing in the unprotected header may be used beyond the integrity
/// protection it has, which is none. The control: the same claims, protected, verify.
#[test]
fn the_cwt_claims_parameter_in_the_unprotected_header_is_refused() {
    let claims = [(uint(1), tstr("https://issuer.example"))];
    assert!(ok(&protected_claims(&claims), &no_unprotected()).is_ok());
    assert_eq!(
        ok(&protected_with(&[]), &map(&[cwt(&claims)])),
        Err(CoseError::CwtClaimsNotProtected)
    );
}

#[test]
fn the_parameter_in_both_headers_is_refused() {
    let claims = [(uint(1), tstr("https://issuer.example"))];
    assert!(ok(&protected_claims(&claims), &map(&[cwt(&claims)])).is_err());
}

#[test]
fn a_duplicate_claim_key_is_refused_even_when_the_values_agree() {
    let key_twice = [
        (uint(1), tstr("https://issuer.example")),
        (uint(1), tstr("https://issuer.example")),
    ];
    assert_eq!(
        ok(&protected_claims(&key_twice), &no_unprotected()),
        Err(CoseError::Malformed("a CWT claim key appears twice"))
    );
    // Keys are compared as values: 1 written as 0x01 and as 0x1801 is one key.
    let one_as_two_bytes = [(uint(1), tstr("a")), (vec![0x18, 0x01], tstr("b"))];
    assert!(ok(&protected_claims(&one_as_two_bytes), &no_unprotected()).is_err());
    // Control: two different keys are fine.
    let different = [(uint(1), tstr("a")), (uint(2), tstr("b"))];
    assert!(ok(&protected_claims(&different), &no_unprotected()).is_ok());
}

#[test]
fn a_duplicate_key_of_an_unread_claim_is_refused_too() {
    let private_twice = [(tstr("x"), uint(1)), (tstr("x"), uint(2))];
    assert!(ok(&protected_claims(&private_twice), &no_unprotected()).is_err());
}

#[test]
fn a_claim_of_the_wrong_type_is_refused() {
    let wrong: [(&str, Pair); 8] = [
        ("iss as a byte string", (uint(1), bstr(b"issuer"))),
        ("iss as an integer", (uint(1), uint(7))),
        ("sub as an array", (uint(2), array(&[]))),
        ("aud as an integer", (uint(3), uint(7))),
        ("aud array with an integer", (uint(3), array(&[uint(7)]))),
        ("exp as text", (uint(4), tstr("tomorrow"))),
        ("nbf as a byte string", (uint(5), bstr(b"x"))),
        // RFC 8392 §5: the value "MUST NOT be prefixed with any CBOR tag", tag 1 included.
        (
            "iat with tag 1",
            (uint(6), [vec![0xc1], uint(1_000)].concat()),
        ),
    ];
    for (what, pair) in wrong {
        assert!(
            matches!(
                ok(&protected_claims(&[pair]), &no_unprotected()),
                Err(CoseError::Malformed(_))
            ),
            "{what}"
        );
    }
}

#[test]
fn a_parameter_that_is_not_a_map_is_refused() {
    for item in [tstr("claims"), uint(1), vec![0x80], vec![0xf6]] {
        let protected = protected_with(&[(uint(15), item)]);
        assert!(matches!(
            ok(&protected, &no_unprotected()),
            Err(CoseError::Malformed(_))
        ));
    }
}

#[test]
fn a_claim_key_that_is_neither_an_integer_nor_text_is_refused() {
    let protected = protected_claims(&[(bstr(b"k"), uint(1))]);
    assert!(matches!(
        ok(&protected, &no_unprotected()),
        Err(CoseError::Malformed(_))
    ));
}

#[test]
fn claims_that_are_not_understood_are_ignored() {
    let protected = protected_claims(&[
        (uint(7), bstr(b"a cti")),
        (nint(-70_000), tstr("private")),
        (tstr("custom"), map(&[(uint(1), vec![0x80])])),
        (uint(1), tstr("https://issuer.example")),
    ]);
    let claims = ok(&protected, &no_unprotected())
        .expect("verifies")
        .cwt_claims;
    assert_eq!(claims.iss.as_deref(), Some("https://issuer.example"));
    assert_eq!(claims.sub, None);
}

/// The claims are read after the signature: a message whose claims are wrong and whose
/// signature is also wrong is refused, and a message that is only signed wrongly is
/// refused for the signature.
#[test]
fn a_cwt_claims_value_never_changes_what_a_valid_signature_over_other_bytes_means() {
    let protected = protected_claims(&[(uint(1), tstr("https://issuer.example"))]);
    let good = message(&signer(), &protected, &no_unprotected(), b"payload");
    assert!(verify(&good).is_ok());
    let mut tampered = good;
    let last = tampered.len() - 1;
    tampered[last] ^= 1;
    assert_eq!(verify(&tampered), Err(CoseError::Signature));
}
