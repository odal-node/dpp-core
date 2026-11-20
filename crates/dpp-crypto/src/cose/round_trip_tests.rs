//! Signing and verifying under each of the four algorithms, and what each signature
//! looks like.

use super::algorithm::cose_alg;
use super::error::CoseError;
use super::sign1::{ContentType, sign_sign1, verify_sign1};
use super::test_support::{
    ALGORITHMS, key_pair, message, no_unprotected, protected_map, public_key,
};
use crate::jws::SignatureLengthError;

const KID: &[u8] = b"did:example:issuer#key-1";

#[test]
fn every_algorithm_signs_and_verifies() {
    for (name, alg) in ALGORITHMS {
        let key = key_pair(name);
        assert_eq!(cose_alg(key.algorithm()), alg, "{name}");
        let signed = sign_sign1(
            &key,
            b"the credential",
            KID,
            Some("application/vc+cose"),
            Some("application/vc"),
        )
        .unwrap();
        assert_eq!(signed[0], 0xd2, "{name}: tag 18");
        let verified = verify_sign1(&signed, &public_key(name)).unwrap();
        assert_eq!(verified.payload, b"the credential", "{name}");
        assert_eq!(verified.kid.as_deref(), Some(KID), "{name}");
        assert_eq!(
            verified.typ,
            Some(ContentType::Text("application/vc+cose".to_owned())),
            "{name}"
        );
        assert_eq!(
            verified.content_type,
            Some(ContentType::Text("application/vc".to_owned())),
            "{name}"
        );
    }
}

#[test]
fn typ_and_content_type_are_optional() {
    for (name, _) in ALGORITHMS {
        let signed = sign_sign1(&key_pair(name), b"x", KID, None, None).unwrap();
        let verified = verify_sign1(&signed, &public_key(name)).unwrap();
        assert_eq!(
            (verified.typ, verified.content_type),
            (None, None),
            "{name}"
        );
    }
}

#[test]
fn an_empty_payload_is_a_payload_and_not_a_detached_one() {
    let signed = sign_sign1(&key_pair("p256"), b"", KID, None, None).unwrap();
    assert_eq!(
        verify_sign1(&signed, &public_key("p256")).unwrap().payload,
        b""
    );
}

/// The signatures are deterministic (Ed25519, and ECDSA by RFC 6979), so the same
/// inputs give the same message.
#[test]
fn signing_is_deterministic() {
    for (name, _) in ALGORITHMS {
        let key = key_pair(name);
        let sign = || sign_sign1(&key, b"payload", KID, Some("t"), Some("c")).unwrap();
        assert_eq!(sign(), sign(), "{name}");
    }
}

/// The protected header is written in ascending label order with definite lengths,
/// and the unprotected header is the empty map.
#[test]
fn the_message_has_the_shape_rfc_9052_describes() {
    let signed = sign_sign1(&key_pair("p256"), b"p", b"k", Some("T"), Some("C")).unwrap();
    // 18( [ h'a4 01 26 03 41 43 04 41 6b 10 41 54', {}, h'70', h'...' ] )
    let expected_start = [
        0xd2, 0x84, // tag 18, array of 4
        0x4c, // the protected header, a byte string of 12
        0xa4, // a map of 4
        0x01, 0x26, // alg: -7
        0x03, 0x61, b'C', // content type
        0x04, 0x41, b'k', // kid
        0x10, 0x61, b'T', // typ
        0xa0, // the unprotected header: the empty map
        0x41, b'p', // the payload
        0x58, 0x40, // the signature, 64 bytes
    ];
    assert_eq!(signed[..expected_start.len()], expected_start);
    assert_eq!(signed.len(), expected_start.len() + 64);
}

#[test]
fn the_signature_lengths_are_those_of_each_algorithm() {
    for ((name, _), expected) in ALGORITHMS.iter().zip([64, 64, 96, 132]) {
        let signed = sign_sign1(&key_pair(name), b"p", KID, None, None).unwrap();
        let tail = &signed[signed.len() - expected - 2..];
        assert_eq!(tail[0], 0x58, "{name}");
        assert_eq!(usize::from(tail[1]), expected, "{name}");
    }
}

#[test]
fn a_signature_of_the_wrong_length_is_a_length_error() {
    let key = key_pair("p384");
    let protected = protected_map(-35);
    // A P-384 key signed this, and then the signature loses a byte.
    let mut signed = message(&key, &protected, &no_unprotected(), b"payload");
    let len = signed.len();
    assert_eq!(&signed[len - 98..len - 96], &[0x58, 96]);
    signed[len - 97] = 95;
    signed.pop();
    let error = verify_sign1(&signed, &public_key("p384")).unwrap_err();
    assert_eq!(
        error,
        CoseError::SignatureLength(SignatureLengthError {
            algorithm: crate::jws::KeyAlgorithm::EcdsaP384,
            expected: 96,
            actual: 95,
        })
    );
}

// ── Low-S on signing, both on verifying ───────────────────────────────────────

/// The signature of `message`, which is its last item: a byte string of `n` bytes.
fn signature_of(message: &[u8], n: usize) -> &[u8] {
    &message[message.len() - n..]
}

/// The message with its last `n` bytes, the signature, replaced by `signature`.
fn with_signature(message: &[u8], n: usize, signature: &[u8]) -> Vec<u8> {
    assert_eq!(signature.len(), n);
    let mut out = message[..message.len() - n].to_vec();
    out.extend_from_slice(signature);
    out
}

/// Replaces the `S` of an `R ‖ S` ECDSA signature by `n − S`, the other valid value.
macro_rules! other_s {
    ($curve:ident, $signature:expr) => {{
        let parsed = $curve::ecdsa::Signature::from_slice($signature).expect("a signature");
        let (r, s) = parsed.split_scalars();
        $curve::ecdsa::Signature::from_scalars(r.to_bytes(), (-*s).to_bytes())
            .expect("the negated scalar")
            .to_bytes()
            .to_vec()
    }};
}

/// `n / 2`, rounded down, of each curve's group order, big-endian: an `S` above it is
/// "high".
fn half_order(name: &str) -> Vec<u8> {
    let hex = match name {
        "p256" => "7fffffff800000007fffffffffffffffde737d56d38bcf4279dce5617e3192a8",
        "p384" => concat!(
            "7fffffffffffffffffffffffffffffffffffffffffffffff",
            "e3b1a6c0fa1b96efac0d06d9245853bd76760cb5666294b9"
        ),
        _ => concat!(
            "00ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            "fd28c343c1df97cb35bfe600a47b84d2e81ddae4dc44ce23d75db7db8f489c3204"
        ),
    };
    hex::decode(hex).unwrap()
}

macro_rules! low_and_high_case {
    ($name:literal, $curve:ident, $n:literal) => {{
        let signed = sign_sign1(&key_pair($name), b"payload", KID, None, None).unwrap();
        let public = public_key($name);
        let signature = signature_of(&signed, $n);
        // What this crate writes is low-S ...
        assert!(
            signature[$n / 2..] <= half_order($name)[..],
            "{}: written low",
            $name
        );
        // ... and its high-S twin is a different signature that verifies as well.
        let twin = other_s!($curve, signature);
        assert_ne!(twin, signature);
        assert!(
            twin[$n / 2..] > half_order($name)[..],
            "{}: the twin is high",
            $name
        );
        assert!(verify_sign1(&signed, &public).is_ok(), "{}: low-S", $name);
        assert!(
            verify_sign1(&with_signature(&signed, $n, &twin), &public).is_ok(),
            "{}: high-S",
            $name
        );
    }};
}

#[test]
fn ecdsa_signatures_are_written_low_s_and_read_either_way() {
    low_and_high_case!("p256", p256, 64);
    low_and_high_case!("p384", p384, 96);
    low_and_high_case!("p521", p521, 132);
}

/// Every signature this crate writes has `S` in the lower half, over many messages.
#[test]
fn signing_always_writes_the_low_half() {
    for (name, n) in [("p256", 64_usize), ("p384", 96), ("p521", 132)] {
        let key = key_pair(name);
        let half = half_order(name);
        for i in 0..40_u8 {
            let signed = sign_sign1(&key, &[i], KID, None, None).unwrap();
            assert!(
                signature_of(&signed, n)[n / 2..] <= half[..],
                "{name} message {i}"
            );
        }
    }
}
