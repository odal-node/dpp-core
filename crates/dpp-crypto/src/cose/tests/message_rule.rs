//! The message rules of `verify_sign1`: the array and its payload, indefinite lengths, unknown labels, and the bytes the signature covers, on a message whose signature is valid.
//!
//! The builders sign whatever protected bytes they are given, so in every test below
//! the signature is good and the one thing wrong with the message is the rule under
//! test. Each rule has a control, the same message without the defect, which must
//! verify: a rule that refused everything would otherwise pass.

use super::super::error::CoseError;
use super::super::sign1::verify_sign1;
use super::super::test_support::{
    NULL, P256_ALG, array, bstr, from_items, indefinite_map, map, message, nint, no_unprotected,
    ok, ordinary, protected_with, signer, tag, tstr, uint, verify,
};

// ── The payload and the array ─────────────────────────────────────────────────

#[test]
fn a_detached_payload_is_refused() {
    let protected = protected_with(&[]);
    let nil = from_items(&signer(), &protected, &no_unprotected(), &NULL, b"");
    assert_eq!(verify(&nil), Err(CoseError::DetachedPayload));
    // The control: the same message with a byte-string payload.
    let present = from_items(&signer(), &protected, &no_unprotected(), &bstr(b"x"), b"x");
    assert!(verify(&present).is_ok());
}

#[test]
fn a_payload_that_is_not_a_byte_string_is_refused() {
    let protected = protected_with(&[]);
    for item in [tstr("text"), uint(1), array(&[]), vec![0xf7]] {
        let message = from_items(&signer(), &protected, &no_unprotected(), &item, b"");
        assert!(
            matches!(verify(&message), Err(CoseError::Malformed(_))),
            "{item:?}"
        );
    }
}

#[test]
fn bytes_after_the_message_are_refused() {
    let mut message = ordinary(&signer(), P256_ALG);
    assert!(verify(&message).is_ok());
    message.push(0x00);
    assert_eq!(verify(&message), Err(CoseError::TrailingBytes));
    // A second message after the first is trailing bytes too.
    let two = [ordinary(&signer(), P256_ALG), ordinary(&signer(), P256_ALG)].concat();
    assert_eq!(verify(&two), Err(CoseError::TrailingBytes));
}

#[test]
fn bytes_inside_the_protected_byte_string_after_its_map_are_refused() {
    let mut protected = protected_with(&[]);
    protected.push(0x00);
    assert!(matches!(
        ok(&protected, &no_unprotected()),
        Err(CoseError::Malformed(_))
    ));
}

#[test]
fn an_array_that_is_not_four_items_is_refused() {
    let key = signer();
    let protected = protected_with(&[]);
    let full = ordinary(&key, P256_ALG);
    // Three items: the signature is missing. Five: one more than there should be.
    let items = |count: usize| {
        let all = [
            bstr(&protected),
            no_unprotected(),
            bstr(b"payload"),
            bstr(&[0; 64]),
            uint(0),
        ];
        tag(18, &array(&all[..count]))
    };
    assert_eq!(verify(&items(3)), Err(CoseError::ArrayLength));
    assert_eq!(verify(&items(5)), Err(CoseError::ArrayLength));
    assert_eq!(verify(&items(0)), Err(CoseError::ArrayLength));
    assert!(verify(&full).is_ok());
}

#[test]
fn something_that_is_not_an_array_is_refused() {
    for item in [map(&[]), bstr(b"abcd"), uint(4), tstr("abcd"), vec![0xf6]] {
        assert!(
            matches!(verify(&item), Err(CoseError::Malformed(_))),
            "{item:?}"
        );
        assert!(matches!(
            verify(&tag(18, &item)),
            Err(CoseError::Malformed(_))
        ));
    }
    assert!(matches!(verify(&[]), Err(CoseError::Malformed(_))));
}

#[test]
fn a_tag_other_than_18_is_refused_and_18_and_none_are_accepted() {
    let tagged = ordinary(&signer(), P256_ALG);
    assert!(verify(&tagged).is_ok());
    assert!(verify(&tagged[1..]).is_ok(), "untagged");
    for number in [0_u64, 17, 19, 98, 1000] {
        let other = tag(number, &tagged[1..]);
        assert_eq!(verify(&other), Err(CoseError::WrongTag), "tag {number}");
    }
    // Tag 18 twice is not a COSE_Sign1 array under the first.
    assert!(matches!(
        verify(&tag(18, &tagged)),
        Err(CoseError::Malformed(_))
    ));
}

#[test]
fn the_protected_header_must_be_a_byte_string() {
    let unwrapped = tag(
        18,
        &array(&[
            protected_with(&[]),
            no_unprotected(),
            bstr(b"p"),
            bstr(&[0; 64]),
        ]),
    );
    assert!(matches!(verify(&unwrapped), Err(CoseError::Malformed(_))));
}

#[test]
fn the_unprotected_header_must_be_a_map() {
    let message = tag(
        18,
        &array(&[
            bstr(&protected_with(&[])),
            bstr(&[]),
            bstr(b"p"),
            bstr(&[0; 64]),
        ]),
    );
    assert!(matches!(verify(&message), Err(CoseError::Malformed(_))));
}

// ── Indefinite lengths ────────────────────────────────────────────────────────

#[test]
fn an_indefinite_length_protected_map_is_refused() {
    let protected = indefinite_map(&[(uint(1), nint(P256_ALG))]);
    assert_eq!(
        ok(&protected, &no_unprotected()),
        Err(CoseError::IndefiniteLength)
    );
    // The control: the same pairs, definite.
    assert!(ok(&map(&[(uint(1), nint(P256_ALG))]), &no_unprotected()).is_ok());
}

#[test]
fn an_indefinite_length_unprotected_map_is_refused() {
    let unprotected = indefinite_map(&[(uint(99), uint(1))]);
    assert_eq!(
        ok(&protected_with(&[]), &unprotected),
        Err(CoseError::IndefiniteLength)
    );
}

#[test]
fn an_indefinite_length_array_is_refused() {
    let signed = ordinary(&signer(), P256_ALG);
    let mut indefinite = vec![0xd2, 0x9f];
    indefinite.extend_from_slice(&signed[2..]);
    indefinite.push(0xff);
    assert_eq!(verify(&indefinite), Err(CoseError::IndefiniteLength));
}

#[test]
fn indefinite_byte_strings_are_refused_where_a_byte_string_is_read() {
    // The payload as `5f 41 'p' ff`.
    let protected = protected_with(&[]);
    let chunked = vec![0x5f, 0x41, b'p', 0xff];
    let message = from_items(&signer(), &protected, &no_unprotected(), &chunked, b"p");
    assert_eq!(verify(&message), Err(CoseError::IndefiniteLength));
}

// ── Unknown labels are stepped over ───────────────────────────────────────────

/// Values of every kind, nested, which an unknown label may carry.
fn nested_values() -> Vec<Vec<u8>> {
    vec![
        uint(7),
        nint(-1_000_000),
        bstr(&[1, 2, 3]),
        tstr("text"),
        array(&[uint(1), array(&[tstr("a"), map(&[(uint(1), bstr(&[]))])])]),
        map(&[(tstr("k"), array(&[NULL.to_vec(), vec![0xf5], vec![0xf4]]))]),
        tag(1, &uint(1_700_000_000)),
        tag(32, &tstr("https://example.org")),
        vec![0xfb, 0x3f, 0xf0, 0, 0, 0, 0, 0, 0],
        vec![0xf9, 0x3c, 0x00],
        // An indefinite-length value is well formed CBOR, and is stepped over.
        vec![0x9f, 0x01, 0x82, 0x02, 0x03, 0xff],
        vec![0xbf, 0x61, b'a', 0x01, 0xff],
        vec![0x5f, 0x41, 0x01, 0x41, 0x02, 0xff],
        vec![0x7f, 0x61, b'a', 0x61, b'b', 0xff],
    ]
}

#[test]
fn an_unknown_label_carrying_a_nested_value_is_stepped_over_in_either_header() {
    for value in nested_values() {
        for label in [uint(99), nint(-70_000), tstr("unknown")] {
            let protected = protected_with(&[(label.clone(), value.clone())]);
            let verified = ok(&protected, &no_unprotected())
                .unwrap_or_else(|e| panic!("protected {label:?} {value:?}: {e}"));
            assert_eq!(verified.payload, b"payload");
            assert_eq!(verified.kid.as_deref(), Some(&b"key-1"[..]));

            let unprotected = map(&[(label.clone(), value.clone())]);
            assert!(
                ok(&protected_with(&[]), &unprotected).is_ok(),
                "unprotected {label:?} {value:?}"
            );
        }
    }
}

/// An unknown value that is not well formed is refused, and so is one nested past the
/// codec's limit of 256 levels.
#[test]
fn an_unknown_value_that_is_not_well_formed_is_refused() {
    for broken in [
        vec![0x82, 0x01],                   // an array of two with one item
        vec![0x61],                         // a text string of one byte, missing
        vec![0x1c],                         // a reserved additional information
        vec![0xff],                         // a break outside an indefinite item
        vec![0xa1, 0x01],                   // a map with a key and no value
        vec![0x9f, 0x01],                   // an indefinite array never closed
        vec![0x1b, 0, 0, 0],                // an integer cut short
        vec![0x5a, 0xff, 0xff, 0xff, 0xff], // a byte string longer than the input
    ] {
        let protected = protected_with(&[(uint(99), broken.clone())]);
        assert!(
            matches!(
                ok(&protected, &no_unprotected()),
                Err(CoseError::Malformed(_))
            ),
            "{broken:?}"
        );
    }
}

#[test]
fn an_unknown_value_nested_too_deeply_is_refused_and_one_shallow_enough_is_not() {
    let nested = |depth: usize| [vec![0x81; depth], uint(0)].concat();
    assert!(
        ok(
            &protected_with(&[(uint(99), nested(100))]),
            &no_unprotected()
        )
        .is_ok()
    );
    for depth in [300, 10_000, 1_000_000] {
        let protected = protected_with(&[(uint(99), nested(depth))]);
        assert!(
            matches!(
                ok(&protected, &no_unprotected()),
                Err(CoseError::Malformed(_))
            ),
            "depth {depth}"
        );
    }
}

/// A claimed length of two to the sixty-four, with nothing behind it, costs nothing.
#[test]
fn an_enormous_claimed_length_allocates_nothing_and_is_refused() {
    let huge_map = [vec![0xbb], vec![0xff; 8]].concat();
    let huge_array = [vec![0x9b], vec![0xff; 8]].concat();
    let huge_bytes = [vec![0x5b], vec![0xff; 8]].concat();
    for header in [huge_map, huge_array, huge_bytes] {
        let protected = protected_with(&[(uint(99), header.clone())]);
        assert!(
            matches!(
                ok(&protected, &no_unprotected()),
                Err(CoseError::Malformed(_))
            ),
            "{header:?}"
        );
        // The same as the protected byte string's whole content.
        let message = tag(18, &[vec![0x84], header.clone()].concat());
        assert!(verify(&message).is_err());
    }
}

// ── The signature and the bytes it covers ─────────────────────────────────────

#[test]
fn a_changed_payload_protected_header_or_signature_is_refused() {
    let signed = ordinary(&signer(), P256_ALG);
    assert!(verify(&signed).is_ok());
    // The payload "payload" sits before the signature's 66 bytes (a two-byte header and
    // 64 bytes).
    let at_payload = signed.len() - 66 - 7;
    assert_eq!(&signed[at_payload..at_payload + 7], b"payload");
    let mut changed = signed.clone();
    changed[at_payload] ^= 0x01;
    assert_eq!(verify(&changed), Err(CoseError::Signature), "the payload");
    let mut changed = signed.clone();
    *changed.last_mut().unwrap() ^= 0x01;
    assert_eq!(verify(&changed), Err(CoseError::Signature), "the signature");
    // The protected header: the last byte of the `typ` it ends with, which sits before
    // the unprotected header (one byte) and the payload's header (one byte).
    let mut changed = signed.clone();
    changed[at_payload - 3] ^= 0x01;
    assert_eq!(
        verify(&changed),
        Err(CoseError::Signature),
        "the protected header"
    );
}

/// The signature covers the protected header as it was received. A header the
/// signer wrote with a longer encoding verifies, and the same header re-encoded
/// shortest does not.
#[test]
fn the_signature_covers_the_protected_header_as_received() {
    // `alg` and `kid` with their labels written as 0x1801 and 0x1804.
    let longer = [
        vec![0xa2, 0x18, 0x01],
        nint(P256_ALG),
        vec![0x18, 0x04],
        bstr(b"key-1"),
    ]
    .concat();
    let key = signer();
    let signed = message(&key, &longer, &no_unprotected(), b"payload");
    let verified = verify(&signed).expect("a longer encoding is read");
    assert_eq!(verified.kid.as_deref(), Some(&b"key-1"[..]));

    let shortest = map(&[(uint(1), nint(P256_ALG)), (uint(4), bstr(b"key-1"))]);
    let reencoded = [
        vec![0xd2, 0x84],
        bstr(&shortest),
        signed_tail(&signed, &longer),
    ]
    .concat();
    assert_eq!(verify(&reencoded), Err(CoseError::Signature));
}

/// The items of `signed` after its protected byte string.
fn signed_tail(signed: &[u8], protected: &[u8]) -> Vec<u8> {
    let header_len = 2 + bstr(protected).len();
    signed[header_len..].to_vec()
}

/// Integers and lengths written longer than they need to be are read.
#[test]
fn longer_encodings_of_lengths_are_read() {
    let signed = ordinary(&signer(), P256_ALG);
    // The array's length as `98 04` instead of `84`.
    let longer = [vec![0xd2, 0x98, 0x04], signed[2..].to_vec()].concat();
    assert!(verify(&longer).is_ok());
    // The tag as `d8 12` instead of `d2`.
    let longer = [vec![0xd8, 0x12], signed[1..].to_vec()].concat();
    assert!(verify(&longer).is_ok());
}

#[test]
fn a_message_under_the_wrong_key_does_not_verify() {
    let signed = ordinary(&signer(), P256_ALG);
    // Another P-256 key.
    let other = crate::jws::PublicKey::from_jwk(&serde_json::json!({
        "kty": "EC",
        "crv": "P-256",
        "x": "TDLl-iSYO_k_A2EebvhGxPQH1ti7XGi4qd4E0CJfulc",
        "y": "8rwdd8GZ_LOV91pOg_34PHO7QQA4ETLDhrGnvoKrb3o",
    }))
    .unwrap();
    assert_eq!(verify_sign1(&signed, &other), Err(CoseError::Signature));
}

/// An error never carries the message, so it can be logged.
#[test]
fn errors_name_rules_and_carry_no_message_bytes() {
    let secret_looking = b"super-secret-payload";
    let protected = protected_with(&[(uint(99), uint(1)), (uint(99), uint(2))]);
    let message = message(&signer(), &protected, &no_unprotected(), secret_looking);
    let error = verify(&message).unwrap_err();
    let shown = format!("{error} {error:?}");
    assert!(!shown.contains("secret"));
}
