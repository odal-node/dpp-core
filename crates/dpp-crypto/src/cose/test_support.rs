//! Shared fixtures and message builders for the COSE tests.
//!
//! The builders assemble a `COSE_Sign1` message from parts, signing the
//! `Sig_structure` of whatever protected bytes and payload they are given. A rule
//! test therefore uses a message whose signature is valid, and the only thing wrong
//! with it is the rule it breaks.

use base64::Engine;
use serde_json::Value;

use crate::cose::error::CoseError;
use crate::cose::sign1::{VerifiedSign1, to_be_signed, verify_sign1};
use crate::jws::PublicKey;
use crate::keystore::KeyEntry;

// ── Suite inputs ──────────────────────────────────────────────────────────────

/// Every vendored file, by its name in the suite.
pub(super) const FILES: [(&str, &str); 12] = [
    (
        "credential-cose-minimal.txt",
        include_str!("fixtures/credential-cose-minimal.txt"),
    ),
    (
        "credential-cose-minimal-not-base64.txt",
        include_str!("fixtures/credential-cose-minimal-not-base64.txt"),
    ),
    (
        "credential-cose-bad-signature.txt",
        include_str!("fixtures/credential-cose-bad-signature.txt"),
    ),
    (
        "credential-cose-bad-media-type.txt",
        include_str!("fixtures/credential-cose-bad-media-type.txt"),
    ),
    (
        "presentation-cose-single.txt",
        include_str!("fixtures/presentation-cose-single.txt"),
    ),
    (
        "presentation-cose-bad-credential.txt",
        include_str!("fixtures/presentation-cose-bad-credential.txt"),
    ),
    (
        "presentation-cose-bad-media-type.txt",
        include_str!("fixtures/presentation-cose-bad-media-type.txt"),
    ),
    ("vm-p256.json", include_str!("fixtures/vm-p256.json")),
    ("vm-p384.json", include_str!("fixtures/vm-p384.json")),
    ("vm-p521.json", include_str!("fixtures/vm-p521.json")),
    ("vm-ed25519.json", include_str!("fixtures/vm-ed25519.json")),
    ("LICENSE.md", include_str!("fixtures/LICENSE.md")),
];

/// The text of the vendored file `name`.
pub(super) fn file(name: &str) -> &'static str {
    FILES
        .iter()
        .find(|(file, _)| *file == name)
        .unwrap_or_else(|| panic!("{name} is not a vendored file"))
        .1
}

/// A suite input read as the standard-alphabet, unpadded base64 the suite writes.
pub(super) fn input(name: &str) -> Vec<u8> {
    base64::engine::general_purpose::STANDARD_NO_PAD
        .decode(file(name).trim_end())
        .unwrap_or_else(|e| panic!("{name} is not base64: {e}"))
}

/// The key file of `algorithm`: `p256`, `p384`, `p521` or `ed25519`.
fn key_file(algorithm: &str) -> Value {
    serde_json::from_str(file(&format!("vm-{algorithm}.json"))).expect("a key file")
}

/// The suite's public key of `algorithm`.
pub(super) fn public_key(algorithm: &str) -> PublicKey {
    PublicKey::from_jwk(&key_file(algorithm)["publicKeyJwk"]).expect("a public key")
}

/// The suite's key pair of `algorithm`.
pub(super) fn key_pair(algorithm: &str) -> KeyEntry {
    KeyEntry::from_private_jwk(&key_file(algorithm)["secretKeyJwk"]).expect("a private key")
}

/// The four algorithms' file names, with each one's COSE `alg`.
pub(super) const ALGORITHMS: [(&str, i64); 4] =
    [("ed25519", -8), ("p256", -7), ("p384", -35), ("p521", -36)];

// ── CBOR written by hand, so that a test can write what the library would not ─

fn head(major: u8, argument: u64) -> Vec<u8> {
    let major = major << 5;
    if argument < 24 {
        vec![major | u8::try_from(argument).expect("below 24")]
    } else if let Ok(argument) = u8::try_from(argument) {
        vec![major | 24, argument]
    } else if let Ok(argument) = u16::try_from(argument) {
        [&[major | 25][..], &argument.to_be_bytes()].concat()
    } else if let Ok(argument) = u32::try_from(argument) {
        [&[major | 26][..], &argument.to_be_bytes()].concat()
    } else {
        [&[major | 27][..], &argument.to_be_bytes()].concat()
    }
}

/// An unsigned integer, shortest form.
pub(super) fn uint(value: u64) -> Vec<u8> {
    head(0, value)
}

/// A negative integer, shortest form.
pub(super) fn nint(value: i64) -> Vec<u8> {
    assert!(value < 0);
    head(1, u64::try_from(-1 - value).expect("fits"))
}

/// A definite-length byte string.
pub(super) fn bstr(bytes: &[u8]) -> Vec<u8> {
    [head(2, bytes.len() as u64), bytes.to_vec()].concat()
}

/// A definite-length text string.
pub(super) fn tstr(text: &str) -> Vec<u8> {
    [head(3, text.len() as u64), text.as_bytes().to_vec()].concat()
}

/// A definite-length array of encoded items.
pub(super) fn array(items: &[Vec<u8>]) -> Vec<u8> {
    [head(4, items.len() as u64), items.concat()].concat()
}

/// A definite-length map of encoded (key, value) pairs, in the order given.
pub(super) fn map(pairs: &[(Vec<u8>, Vec<u8>)]) -> Vec<u8> {
    let body: Vec<u8> = pairs
        .iter()
        .flat_map(|(k, v)| [k.clone(), v.clone()].concat())
        .collect();
    [head(5, pairs.len() as u64), body].concat()
}

/// An indefinite-length map of encoded (key, value) pairs.
pub(super) fn indefinite_map(pairs: &[(Vec<u8>, Vec<u8>)]) -> Vec<u8> {
    let body: Vec<u8> = pairs
        .iter()
        .flat_map(|(k, v)| [k.clone(), v.clone()].concat())
        .collect();
    [vec![0xbf], body, vec![0xff]].concat()
}

/// A tagged item.
pub(super) fn tag(number: u64, item: &[u8]) -> Vec<u8> {
    [head(6, number), item.to_vec()].concat()
}

/// CBOR `null`.
pub(super) const NULL: [u8; 1] = [0xf6];

// ── Messages ──────────────────────────────────────────────────────────────────

/// A protected header map of `alg`, a `kid` and a `typ`.
pub(super) fn protected_map(alg: i64) -> Vec<u8> {
    map(&[
        (uint(1), if alg < 0 { nint(alg) } else { uint(alg as u64) }),
        (uint(4), bstr(b"key-1")),
        (uint(16), tstr("application/vc+cose")),
    ])
}

/// The empty unprotected header.
pub(super) fn no_unprotected() -> Vec<u8> {
    map(&[])
}

/// The tagged message of `protected` (the **contents** of the protected byte string),
/// `unprotected` (an encoded map) and `payload`, signed by `key` over those exact
/// bytes.
pub(super) fn message(
    key: &KeyEntry,
    protected: &[u8],
    unprotected: &[u8],
    payload: &[u8],
) -> Vec<u8> {
    let signature = key
        .sign(&to_be_signed(protected, payload).expect("a Sig_structure"))
        .expect("an unrevoked key signs");
    tag(
        18,
        &array(&[
            bstr(protected),
            unprotected.to_vec(),
            bstr(payload),
            bstr(&signature),
        ]),
    )
}

/// The message of the ordinary header of `key`'s algorithm.
pub(super) fn ordinary(key: &KeyEntry, algorithm_cose: i64) -> Vec<u8> {
    message(
        key,
        &protected_map(algorithm_cose),
        &no_unprotected(),
        b"payload",
    )
}

/// A message built from already-encoded items, for the rules about the array itself.
/// The signature is the key's over the given protected bytes and payload.
pub(super) fn from_items(
    key: &KeyEntry,
    protected: &[u8],
    unprotected: &[u8],
    payload_item: &[u8],
    payload_bytes: &[u8],
) -> Vec<u8> {
    let signature = key
        .sign(&to_be_signed(protected, payload_bytes).expect("a Sig_structure"))
        .expect("an unrevoked key signs");
    tag(
        18,
        &array(&[
            bstr(protected),
            unprotected.to_vec(),
            payload_item.to_vec(),
            bstr(&signature),
        ]),
    )
}

// ── The rule tests' P-256 message, shared by both rule test files ──────────────

pub(super) const P256_ALG: i64 = -7;

pub(super) fn verify(message: &[u8]) -> Result<VerifiedSign1, CoseError> {
    verify_sign1(message, &public_key("p256"))
}

pub(super) fn signer() -> KeyEntry {
    key_pair("p256")
}

/// The ordinary header, with `extra` pairs added to the protected map.
pub(super) fn protected_with(extra: &[(Vec<u8>, Vec<u8>)]) -> Vec<u8> {
    let mut pairs = vec![
        (uint(1), nint(P256_ALG)),
        (uint(4), bstr(b"key-1")),
        (uint(16), tstr("application/vc+cose")),
    ];
    pairs.extend_from_slice(extra);
    map(&pairs)
}

pub(super) fn ok(protected: &[u8], unprotected: &[u8]) -> Result<VerifiedSign1, CoseError> {
    verify(&message(&signer(), protected, unprotected, b"payload"))
}
