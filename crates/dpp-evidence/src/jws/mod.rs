//! JWS compact serialisation verification — EdDSA/Ed25519, algorithm-pinned.
//!
//! Vendored from `dpp-crypto/src/jws/verifier.rs` rather than depended on
//! directly: `dpp-crypto` pulls in `rand`/`argon2`/`aes-gcm` for its keystore
//! and encryption paths, which break the `wasm32-unknown-unknown` build this
//! crate targets. Verification itself needs none of that — only Ed25519
//! signature checking, which is reproduced verbatim in this module. If
//! `dpp-crypto`'s verifier ever changes, mirror the change here too — the
//! cross-verification tests in `dpp-tests/tests/jws_cross_verification.rs`
//! exist to catch drift between the two implementations.

mod verify;

#[cfg(test)]
mod tests;

pub use verify::{
    VerifyError, decode_payload_bytes, extract_key_by_fingerprint, extract_kid_from_jws,
    extract_primary_public_key, resolve_public_key, verify_jws, verify_jws_content,
};
