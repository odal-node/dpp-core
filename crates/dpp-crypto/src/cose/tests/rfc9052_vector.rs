//! The `COSE_Sign1` example of RFC 9052 Appendix C.2.1, decoded and verified under the
//! public key that Appendix C.7.1 gives for its `kid`.
//!
//! C.2 has one example, a single ECDSA signature. C.1 holds `COSE_Sign` examples, which
//! this crate does not read.

use super::super::error::CoseError;
use super::super::sign1::{to_be_signed, verify_sign1};
use super::super::test_support::{bstr, uint};
use crate::jws::{KeyAlgorithm, PublicKey};

/// Appendix C.2.1: `18([h'a10126', {4: '11'}, 'This is the content.', h'8eb3…cb36'])`.
const C21: &str = concat!(
    "d28443a10126a10442313154546869732069732074686520636f6e74656e742e58408eb33e4ca31d1c46",
    "5ab05aac34cc6b23d58fef5c083106c4d25a91aef0b0117e2af9a291aa32e14ab834dc56ed2a2234445",
    "47e01f11d3b0916e5a4c345cacb36",
);

/// The key with `kid` `'11'` in Appendix C.7.1, P-256: `x` then `y`.
const KEY_11: (&str, &str) = (
    "bac5b11cad8f99f9c72b05cf4b9e26d244dc189f745228255a219a86d6a09eff",
    "20138bf82dc1b6d562be0fa54ab7804a3a64b6d72ccfed6b6fb6ed28bbfc117e",
);

/// The key with `kid` `'peregrin.took@tuckborough.example'` in Appendix C.7.1, P-256.
const KEY_PEREGRIN: (&str, &str) = (
    "98f50a4ff6c05861c8860d13a638ea56c3f5ad7590bbfbf054e1c7b4d91d6280",
    "f01400b089867804b8e9fc96c3932161f1934f4223069170d924b7e03bf822bb",
);

fn key((x, y): (&str, &str)) -> PublicKey {
    let sec1 = [
        &[0x04][..],
        &hex::decode(x).unwrap(),
        &hex::decode(y).unwrap(),
    ]
    .concat();
    PublicKey::from_bytes(KeyAlgorithm::EcdsaP256, &sec1).expect("a P-256 point")
}

fn c21() -> Vec<u8> {
    hex::decode(C21).expect("hex")
}

#[test]
fn the_example_is_98_bytes_as_the_rfc_says() {
    assert_eq!(c21().len(), 98);
}

#[test]
fn the_example_decodes_and_verifies_under_the_key_of_appendix_c7() {
    let verified = verify_sign1(&c21(), &key(KEY_11)).expect("the RFC's example verifies");
    assert_eq!(verified.payload, b"This is the content.");
    // The `kid` of this example is in the unprotected header, which nothing
    // authenticates, so it is not returned.
    assert_eq!(verified.kid, None);
    assert_eq!(verified.typ, None);
    assert_eq!(verified.content_type, None);
}

#[test]
fn the_untagged_form_verifies_too() {
    let message = c21();
    assert_eq!(message[0], 0xd2, "tag 18");
    let verified = verify_sign1(&message[1..], &key(KEY_11)).expect("untagged");
    assert_eq!(verified.payload, b"This is the content.");
}

#[test]
fn another_key_of_the_same_appendix_does_not_verify_it() {
    assert_eq!(
        verify_sign1(&c21(), &key(KEY_PEREGRIN)),
        Err(CoseError::Signature)
    );
}

#[test]
fn a_changed_payload_does_not_verify() {
    let mut message = c21();
    // The payload starts after: tag, array, protected (4 bytes), unprotected (5), and
    // its own header byte.
    let first_payload_byte = 2 + 4 + 5 + 1;
    assert_eq!(message[first_payload_byte], b'T');
    message[first_payload_byte] = b't';
    assert_eq!(
        verify_sign1(&message, &key(KEY_11)),
        Err(CoseError::Signature)
    );
}

/// The bytes that are signed are `["Signature1", h'a10126', h'', payload]` in definite
/// lengths with minimal arguments (RFC 9052 §4.4 and §9).
#[test]
fn the_signed_bytes_are_the_sig_structure_in_minimal_definite_form() {
    let expected = [
        vec![0x84],
        vec![0x6a],
        b"Signature1".to_vec(),
        vec![0x43, 0xa1, 0x01, 0x26],
        vec![0x40],
        vec![0x54],
        b"This is the content.".to_vec(),
    ]
    .concat();
    assert_eq!(
        to_be_signed(&[0xa1, 0x01, 0x26], b"This is the content.").unwrap(),
        expected
    );
}

/// The length argument of the payload is the shortest, at each width.
#[test]
fn the_payload_length_in_the_signed_bytes_is_the_shortest() {
    for (len, head) in [
        (0_usize, bstr(&[])),
        (23, bstr(&[0; 23])),
        (24, bstr(&[0; 24])),
        (255, bstr(&[0; 255])),
        (256, bstr(&[0; 256])),
        (65_535, bstr(&[0; 65_535])),
        (65_536, bstr(&[0; 65_536])),
    ] {
        let signed = to_be_signed(&[], &vec![0; len]).unwrap();
        // 0x84, "Signature1" (11 bytes), h'' (1), h'' (1), then the payload item.
        assert_eq!(signed[0], 0x84);
        assert_eq!(&signed[1..12], &[&[0x6a][..], b"Signature1"].concat()[..]);
        assert_eq!(&signed[12..14], &[0x40, 0x40]);
        assert_eq!(&signed[14..], &head[..], "payload of {len} bytes");
    }
    // `uint` is the shortest form too, which the other tests lean on.
    assert_eq!(uint(24), vec![0x18, 24]);
}
