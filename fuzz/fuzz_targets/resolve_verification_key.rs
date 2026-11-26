#![no_main]
//! Fuzz key resolution from a DID document: the `kid` in a token's header names a
//! verification method in a document that arrives beside the token, and both come
//! from another operator as untrusted text. Property: resolving the key and then
//! verifying the token under it never panic, whatever the document and the token
//! hold.

use dpp_crypto::jws::{resolve_verification_key, verify_jws};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // The DID document is every byte before the first zero byte; the token is
    // every byte after it.
    let Some(separator) = data.iter().position(|&byte| byte == 0) else {
        return;
    };
    let (document_bytes, jws_bytes) = (&data[..separator], &data[separator + 1..]);
    let Ok(document) = serde_json::from_slice::<serde_json::Value>(document_bytes) else {
        return;
    };
    let Ok(jws) = std::str::from_utf8(jws_bytes) else {
        return;
    };
    if let Some(key) = resolve_verification_key(&document, jws) {
        let _ = verify_jws(jws, &key);
    }
});
