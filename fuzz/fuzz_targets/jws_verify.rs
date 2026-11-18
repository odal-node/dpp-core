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

/// Published public keys, one of each algorithm, so verification reaches the signature
/// check: RFC 8037 appendix A.2's Ed25519 key, RFC 7515 appendix A.3.1's P-256 key,
/// RFC 6979 clause A.2.6's P-384 point, and RFC 7515 appendix A.4's P-521 key. Only
/// the public parts are used, and the fuzzer holds no private half.
static KEYS: LazyLock<[PublicKey; 4]> = LazyLock::new(|| {
    let key = |jwk: serde_json::Value| PublicKey::from_jwk(&jwk).expect("a public key");
    [
        key(serde_json::json!({
            "kty": "OKP",
            "crv": "Ed25519",
            "x": "11qYAYKxCrfVS_7TyWQHOg7hcvPapiMlrwIaaPcHURo",
        })),
        key(serde_json::json!({
            "kty": "EC",
            "crv": "P-256",
            "x": "f83OJ3D2xF1Bg8vub9tLe1gHMzV76e8Tus9uPHvRVEU",
            "y": "x_FEzRu9m36HLN_tue659LNpXW6pCyStikYjKIWI5a0",
        })),
        key(serde_json::json!({
            "kty": "EC",
            "crv": "P-384",
            "x": "7DpOQVtOGaRWhhgCn0J_pdqai8SukuAuBqrlKGswDGTe-PDqkFWGYGSiVFFUgLwT",
            "y": "gBXZty19VyROqO-awMYhiWcIpZNn-d-59UyoSz8cnbEoiyMcOuDU_nNE_SUzJkcg",
        })),
        key(serde_json::json!({
            "kty": "EC",
            "crv": "P-521",
            "x": "AekpBQ8ST8a8VcfVOTNl353vSrDCLLJXmPk06wTjxrrjcBpXp5EOnYG_NjFZ6OvLFV1jSfS9tsz4qUxcWceqwQGk",
            "y": "ADSmRA43Z1DSNx_RvcLI87cdL07l6jQyyBXMoxVg_l2Th-x3S1WDhjDly79ajL4Kkd0AZMaZmh9ubmf63e3kyMj2",
        })),
    ]
});

fuzz_target!(|data: &[u8]| {
    if let Ok(token) = std::str::from_utf8(data) {
        let _ = extract_kid_from_jws(token);
        // The fuzzer holds no private key, so nothing it produces is a valid
        // signature under either.
        for key in KEYS.iter() {
            if let Ok(verified) = verify_jws(token, key) {
                assert!(
                    !verified,
                    "verified a token the key never signed: {token:?}"
                );
            }
        }
    }
});
