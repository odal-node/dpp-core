//! The header rules of `verify_sign1`: labels, `alg`, `crit`, `typ` and content type, on a message whose signature is valid.
//!
//! The builders sign whatever protected bytes they are given, so in every test below
//! the signature is good and the one thing wrong with the message is the rule under
//! test. Each rule has a control, the same message without the defect, which must
//! verify: a rule that refused everything would otherwise pass.

use super::error::CoseError;
use super::sign1::{ContentType, verify_sign1};
use super::test_support::{
    NULL, P256_ALG, array, bstr, key_pair, map, message, nint, no_unprotected, ok, ordinary,
    protected_with, public_key, signer, tstr, uint, verify,
};

#[test]
fn control_the_ordinary_message_verifies() {
    let verified = verify(&ordinary(&signer(), P256_ALG)).unwrap();
    assert_eq!(verified.payload, b"payload");
    assert_eq!(verified.kid.as_deref(), Some(&b"key-1"[..]));
}

// ── Labels unique, and in one bucket ──────────────────────────────────────────

#[test]
fn a_label_twice_in_the_protected_header_is_refused() {
    let twice = protected_with(&[(uint(99), uint(1)), (uint(99), uint(2))]);
    assert_eq!(
        ok(&twice, &no_unprotected()),
        Err(CoseError::DuplicateLabel)
    );
    // The control: the same map with one of them.
    assert!(ok(&protected_with(&[(uint(99), uint(1))]), &no_unprotected()).is_ok());
}

#[test]
fn alg_twice_is_refused_even_with_the_same_value() {
    let twice = map(&[(uint(1), nint(P256_ALG)), (uint(1), nint(P256_ALG))]);
    assert_eq!(
        ok(&twice, &no_unprotected()),
        Err(CoseError::DuplicateLabel)
    );
}

/// Labels are compared as values: `1` written `0x01` and `0x1801` is one label.
#[test]
fn a_label_twice_in_two_encodings_is_refused() {
    let twice = [
        vec![0xa2],
        uint(1),
        nint(P256_ALG),
        vec![0x18, 0x01], // `1`, written longer than it needs to be
        nint(P256_ALG),
    ]
    .concat();
    assert_eq!(
        ok(&twice, &no_unprotected()),
        Err(CoseError::DuplicateLabel)
    );
}

#[test]
fn a_text_label_twice_is_refused() {
    let twice = protected_with(&[(tstr("x"), uint(1)), (tstr("x"), uint(2))]);
    assert_eq!(
        ok(&twice, &no_unprotected()),
        Err(CoseError::DuplicateLabel)
    );
    // `"1"` and `1` are different labels.
    let distinct = protected_with(&[(tstr("1"), uint(1))]);
    assert!(ok(&distinct, &no_unprotected()).is_ok());
}

#[test]
fn a_label_twice_in_the_unprotected_header_is_refused() {
    let twice = map(&[(uint(99), uint(1)), (uint(99), uint(2))]);
    assert_eq!(
        ok(&protected_with(&[]), &twice),
        Err(CoseError::DuplicateLabel)
    );
    assert!(ok(&protected_with(&[]), &map(&[(uint(99), uint(1))])).is_ok());
}

#[test]
fn a_label_in_both_buckets_is_refused() {
    // The `kid`, which both buckets may hold, and an unknown label.
    for label in [4_u64, 99] {
        let value = if label == 4 { bstr(b"key-1") } else { uint(1) };
        let unprotected = map(&[(uint(label), value)]);
        assert_eq!(
            ok(&protected_with(&[(uint(99), uint(1))]), &unprotected),
            Err(CoseError::LabelInBothBuckets),
            "label {label}"
        );
    }
    // The control: a label in the unprotected header only.
    assert!(ok(&protected_with(&[]), &map(&[(uint(99), uint(1))])).is_ok());
}

#[test]
fn alg_in_both_buckets_is_refused() {
    let unprotected = map(&[(uint(1), nint(P256_ALG))]);
    assert_eq!(
        ok(&protected_with(&[]), &unprotected),
        Err(CoseError::LabelInBothBuckets)
    );
}

#[test]
fn a_label_that_is_neither_integer_nor_text_is_refused() {
    for label in [
        bstr(b"1"),
        vec![0xf5],
        vec![0xfb, 0x3f, 0xf0, 0, 0, 0, 0, 0, 0],
        array(&[]),
    ] {
        let header = protected_with(&[(label.clone(), uint(1))]);
        assert!(
            matches!(ok(&header, &no_unprotected()), Err(CoseError::Malformed(_))),
            "{label:?}"
        );
    }
}

// ── alg: protected, and the key's ─────────────────────────────────────────────

#[test]
fn alg_only_in_the_unprotected_header_is_refused() {
    let protected = map(&[(uint(4), bstr(b"key-1"))]);
    let unprotected = map(&[(uint(1), nint(P256_ALG))]);
    assert_eq!(
        ok(&protected, &unprotected),
        Err(CoseError::AlgorithmNotProtected)
    );
}

#[test]
fn no_alg_at_all_is_refused() {
    let protected = map(&[(uint(4), bstr(b"key-1"))]);
    assert_eq!(
        ok(&protected, &no_unprotected()),
        Err(CoseError::MissingAlgorithm)
    );
    // An empty protected byte string, and the empty map in one (RFC 9052 §3 has a
    // recipient accept both), carry no alg.
    assert_eq!(ok(&[], &no_unprotected()), Err(CoseError::MissingAlgorithm));
    assert_eq!(
        ok(&map(&[]), &no_unprotected()),
        Err(CoseError::MissingAlgorithm)
    );
}

#[test]
fn an_alg_that_is_not_the_keys_is_refused() {
    // Signed by the P-256 key, but saying another algorithm.
    for alg in [-35, -36, -8, -19, -6, 0, 1] {
        let protected = map(&[(uint(1), if alg < 0 { nint(alg) } else { uint(alg as u64) })]);
        assert_eq!(
            ok(&protected, &no_unprotected()),
            Err(CoseError::AlgorithmMismatch),
            "alg {alg}"
        );
    }
}

#[test]
fn an_alg_that_is_text_is_refused() {
    for alg in ["ES256", "none", "-7"] {
        let protected = map(&[(uint(1), tstr(alg))]);
        assert_eq!(
            ok(&protected, &no_unprotected()),
            Err(CoseError::AlgorithmMismatch),
            "{alg}"
        );
    }
}

#[test]
fn an_alg_of_another_type_is_refused() {
    for alg in [bstr(&[0x26]), array(&[nint(P256_ALG)]), NULL.to_vec()] {
        let protected = map(&[(uint(1), alg)]);
        assert!(matches!(
            ok(&protected, &no_unprotected()),
            Err(CoseError::Malformed(_))
        ));
    }
}

/// An Ed25519 key signs with -8 only. The fully specified -19 of RFC 9864 is not read.
#[test]
fn ed25519_is_eddsa_minus_8_and_nothing_else() {
    let key = key_pair("ed25519");
    let verify_ed = |alg: i64| {
        verify_sign1(
            &message(&key, &map(&[(uint(1), nint(alg))]), &no_unprotected(), b"p"),
            &public_key("ed25519"),
        )
    };
    assert!(verify_ed(-8).is_ok());
    assert_eq!(verify_ed(-19), Err(CoseError::AlgorithmMismatch));
    assert_eq!(verify_ed(-7), Err(CoseError::AlgorithmMismatch));
}

// ── crit, typ, content type ───────────────────────────────────────────────────

#[test]
fn crit_is_refused_in_either_header() {
    let crit = (uint(2), array(&[uint(99)]));
    assert_eq!(
        ok(
            &protected_with(&[crit.clone(), (uint(99), uint(1))]),
            &no_unprotected()
        ),
        Err(CoseError::Critical)
    );
    assert_eq!(
        ok(&protected_with(&[]), &map(&[crit])),
        Err(CoseError::Critical)
    );
    // Even a `crit` that RFC 9052 §3.1 forbids to be written: an empty array.
    assert_eq!(
        ok(&protected_with(&[(uint(2), array(&[]))]), &no_unprotected()),
        Err(CoseError::Critical)
    );
}

#[test]
fn typ_in_the_unprotected_header_is_refused() {
    let protected = map(&[(uint(1), nint(P256_ALG))]);
    let unprotected = map(&[(uint(16), tstr("application/vc+cose"))]);
    assert_eq!(
        ok(&protected, &unprotected),
        Err(CoseError::TypeNotProtected)
    );
}

#[test]
fn typ_and_content_type_are_returned_as_written_for_the_caller_to_judge() {
    let protected = map(&[
        (uint(1), nint(P256_ALG)),
        (uint(3), uint(60)),
        (uint(16), tstr("really/bad")),
    ]);
    let verified = ok(&protected, &no_unprotected()).unwrap();
    assert_eq!(
        verified.typ,
        Some(ContentType::Text("really/bad".to_owned()))
    );
    assert_eq!(verified.content_type, Some(ContentType::CoapFormat(60)));
    assert_eq!(verified.typ.unwrap().as_text(), Some("really/bad"));
}

#[test]
fn typ_and_content_type_must_be_text_or_unsigned() {
    for value in [nint(-1), bstr(b"a"), array(&[]), NULL.to_vec()] {
        for label in [3_u64, 16] {
            let protected = map(&[(uint(1), nint(P256_ALG)), (uint(label), value.clone())]);
            assert!(
                matches!(
                    ok(&protected, &no_unprotected()),
                    Err(CoseError::Malformed(_))
                ),
                "label {label}: {value:?}"
            );
        }
    }
}

#[test]
fn kid_must_be_a_byte_string() {
    let protected = map(&[(uint(1), nint(P256_ALG)), (uint(4), tstr("key-1"))]);
    assert!(matches!(
        ok(&protected, &no_unprotected()),
        Err(CoseError::Malformed(_))
    ));
}

/// A `kid` or content type only in the unprotected header is not returned: nothing
/// authenticates it.
#[test]
fn unprotected_values_are_not_returned() {
    let unprotected = map(&[(uint(4), bstr(b"unsigned")), (uint(3), tstr("a/b"))]);
    let verified = ok(&protected_with(&[]), &unprotected);
    // The protected header has a kid, so this one is in both buckets.
    assert_eq!(verified, Err(CoseError::LabelInBothBuckets));
    let protected = map(&[(uint(1), nint(P256_ALG))]);
    let verified = ok(&protected, &unprotected).unwrap();
    assert_eq!(
        (verified.kid, verified.content_type, verified.typ),
        (None, None, None)
    );
}
