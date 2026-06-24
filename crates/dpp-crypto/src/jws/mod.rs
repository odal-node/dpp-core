//! JWS signing primitives — EdDSA over RFC 8785 (JCS) canonical bytes.

pub mod algorithm;
pub mod canonical;
pub mod signer;
#[cfg(test)]
mod tests;
pub mod verifier;

pub use algorithm::{ED25519_CRV, EDDSA_ALG, is_allowed_alg};
pub use canonical::canonicalize;
pub use signer::{sign, verify};
pub use verifier::{
    extract_key_by_fingerprint, extract_kid_from_jws, extract_primary_public_key, verify_jws,
};
