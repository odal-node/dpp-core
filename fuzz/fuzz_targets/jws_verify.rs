#![no_main]
//! Fuzz compact JWS verification — the signature on a passport fetched from
//! another operator, or carried in a proof file, arrives as untrusted text.
//! Property: `verify_jws` and `extract_kid_from_jws` return `Ok`/`Err` for any
//! token, never panic, and never report a token as verified that the key did
//! not sign — under a key of each algorithm, so the fuzzer can also try to move
//! a signature from one format to another.

use std::sync::LazyLock;

use dpp_crypto::jws::{PublicKey, extract_kid_from_jws, verify_jws};
use libfuzzer_sys::fuzz_target;

/// Public keys whose private keys were discarded when they were generated, one of each
/// algorithm. Real keys, so verification reaches the signature check instead of stopping
/// at key decoding, and ones nobody can sign for, so a published test key cannot make a
/// seeded token verify. The RFC examples' keys are not used: their private halves are
/// published, so a token signed under one of them would verify.
static KEYS: LazyLock<[PublicKey; 4]> = LazyLock::new(|| {
    let key = |jwk: serde_json::Value| PublicKey::from_jwk(&jwk).expect("a public key");
    [
        key(serde_json::json!({
            "kty": "OKP",
            "crv": "Ed25519",
            "x": "m4FM3_AWmxoTjYQMRJa88DtRmizWPMud7nzZ6COI5e8",
        })),
        key(serde_json::json!({
            "kty": "EC",
            "crv": "P-256",
            "x": "TDLl-iSYO_k_A2EebvhGxPQH1ti7XGi4qd4E0CJfulc",
            "y": "8rwdd8GZ_LOV91pOg_34PHO7QQA4ETLDhrGnvoKrb3o",
        })),
        key(serde_json::json!({
            "kty": "EC",
            "crv": "P-384",
            "x": "TrcusmKqkYsYLfBMjPDYFxJa_c1PwrYSw-nSEw1vsAt_FQV8mokOAuFMSC3Ba-Bq",
            "y": "c2krEf1O9jL7pFbpfs3PtmxX2IQZa5HrjftdelSfcD17zQFAMY0iQM8cIglDUcEN",
        })),
        key(serde_json::json!({
            "kty": "EC",
            "crv": "P-521",
            "x": "ACMMllf-qE2oRPusCcmE9JOiG0-0IFbR5lkecH1HSPh2vXb_M-CXpWEdWkx1suJAj61gRUvGg2jrIA9THI6y_JjR",
            "y": "AFf4vAtpjnv0WVz55HU2gWMAeYptbBqdB7rPDqcUkOE2Ql3sjTV_S0v-7KGFRAlK5rElMqaWe-k7rbuHIlDu6oeL",
        })),
    ]
});

fuzz_target!(|data: &[u8]| {
    if let Ok(token) = std::str::from_utf8(data) {
        let _ = extract_kid_from_jws(token);
        // The fuzzer holds no private key, so nothing it produces is a valid
        // signature under any of them.
        for key in KEYS.iter() {
            assert!(
                verify_jws(token, key).is_err(),
                "verified a token the key never signed: {token:?}"
            );
        }
    }
});
