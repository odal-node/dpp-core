//! Property tests for the header path: an unknown label may carry any well-formed
//! value, and no input makes verification panic.

use proptest::prelude::*;

use super::error::CoseError;
use super::sign1::verify_sign1;
use super::test_support::{
    array, bstr, key_pair, map, message, nint, no_unprotected, public_key, tag, tstr, uint,
};

/// An arbitrary well-formed CBOR value, nested up to a few levels. Floats and simple
/// values are the ones the codec reads on a skip.
fn cbor_value() -> impl Strategy<Value = Vec<u8>> {
    let leaf = prop_oneof![
        any::<u64>().prop_map(uint),
        (i64::MIN..0_i64).prop_map(nint),
        prop::collection::vec(any::<u8>(), 0..300).prop_map(|b| bstr(&b)),
        "\\PC{0,40}".prop_map(|s| tstr(&s)),
        Just(vec![0xf4]),
        Just(vec![0xf5]),
        Just(vec![0xf6]),
        Just(vec![0xf7]),
        any::<f64>().prop_map(|f| [vec![0xfb], f.to_be_bytes().to_vec()].concat()),
        any::<f32>().prop_map(|f| [vec![0xfa], f.to_be_bytes().to_vec()].concat()),
    ];
    leaf.prop_recursive(6, 64, 6, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..6).prop_map(|items| array(&items)),
            prop::collection::vec((inner.clone(), inner.clone()), 0..4)
                .prop_map(|pairs| map(&pairs)),
            (any::<u64>(), inner.clone()).prop_map(|(n, item)| tag(n, &item)),
            prop::collection::vec(inner, 0..5)
                .prop_map(|items| { [vec![0x9f], items.concat(), vec![0xff]].concat() }),
        ]
    })
}

proptest! {
    /// Whatever an unknown label carries, in either header, the message verifies and
    /// the values this crate reads are unchanged.
    #[test]
    fn an_unknown_label_may_carry_any_well_formed_value(
        value in cbor_value(),
        label in 100_u64..1_000_000,
        in_protected in any::<bool>(),
    ) {
        let key = key_pair("p256");
        let mut pairs = vec![(uint(1), nint(-7)), (uint(4), bstr(b"kid"))];
        let mut unprotected = no_unprotected();
        if in_protected {
            pairs.push((uint(label), value));
        } else {
            unprotected = map(&[(uint(label), value)]);
        }
        let signed = message(&key, &map(&pairs), &unprotected, b"payload");
        let verified = verify_sign1(&signed, &public_key("p256"));
        prop_assert!(verified.is_ok(), "{:?}", verified);
        let verified = verified.unwrap();
        prop_assert_eq!(verified.payload, b"payload".to_vec());
        prop_assert_eq!(verified.kid, Some(b"kid".to_vec()));
    }

    /// No byte string makes verification panic, and none that this crate did not sign
    /// verifies.
    #[test]
    fn arbitrary_bytes_never_panic_and_never_verify(bytes in prop::collection::vec(any::<u8>(), 0..400)) {
        for algorithm in ["ed25519", "p256", "p384", "p521"] {
            prop_assert!(verify_sign1(&bytes, &public_key(algorithm)).is_err());
        }
    }

    /// Flipping any one bit of a valid message either leaves it verifying (a bit the
    /// signature does not cover, such as the unprotected header's) or refuses it, and
    /// never panics.
    #[test]
    fn one_changed_bit_never_panics(index in 0_usize..200, bit in 0_u8..8) {
        let key = key_pair("p256");
        let signed = message(
            &key,
            &map(&[(uint(1), nint(-7)), (uint(4), bstr(b"kid"))]),
            &map(&[(uint(99), tstr("x"))]),
            b"payload",
        );
        let mut changed = signed.clone();
        let at = index % changed.len();
        changed[at] ^= 1 << bit;
        let result = verify_sign1(&changed, &public_key("p256"));
        if let Ok(verified) = result {
            // Only the unprotected header's value may have moved.
            prop_assert_eq!(verified.payload, b"payload".to_vec());
        } else {
            prop_assert!(!matches!(result, Err(CoseError::Encode)));
        }
    }
}
