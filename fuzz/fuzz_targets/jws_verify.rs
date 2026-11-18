#![no_main]
//! Fuzz compact JWS verification — the signature on a passport fetched from
//! another operator, or carried in a proof file, arrives as untrusted text.
//! Property: `verify_jws` and `extract_kid_from_jws` return `Ok`/`Err` for any
//! token, never panic, and never report a token as verified that the key did
//! not sign.

use std::sync::LazyLock;

use dpp_crypto::jws::{PublicKey, extract_kid_from_jws, verify_jws};
use libfuzzer_sys::fuzz_target;

/// An Ed25519 public key, base64url without padding, whose private key was
/// discarded when it was generated. A real key, so verification reaches the
/// signature check instead of stopping at key decoding, and one nobody can sign
/// for, so a published test key cannot make a seeded token verify.
static KEY: LazyLock<PublicKey> = LazyLock::new(|| {
    PublicKey::from_jwk(&serde_json::json!({
        "kty": "OKP",
        "crv": "Ed25519",
        "x": "m4FM3_AWmxoTjYQMRJa88DtRmizWPMud7nzZ6COI5e8",
    }))
    .expect("the fuzz key is an Ed25519 public key")
});

fuzz_target!(|data: &[u8]| {
    if let Ok(token) = std::str::from_utf8(data) {
        let _ = extract_kid_from_jws(token);
        // The fuzzer holds no private key, so nothing it produces is a valid
        // signature under this one.
        if let Ok(verified) = verify_jws(token, &KEY) {
            assert!(
                !verified,
                "verified a token the key never signed: {token:?}"
            );
        }
    }
});
