//! JWS signing primitives — EdDSA over RFC 8785 (JCS) canonical bytes.

pub mod algorithm;
pub mod canonical;
#[cfg(test)]
mod crit_tests;
#[cfg(test)]
mod proptests;
#[cfg(test)]
mod resolve_tests;
pub mod signer;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod thumbprint_tests;
pub mod verifier;

pub use algorithm::{
    ED25519_ALG, ED25519_CRV, EDDSA_ALG, JWK_THUMBPRINT_URI_PREFIX, is_allowed_alg,
};
pub use canonical::canonicalize;
pub use signer::{sign, sign_typed, verify};
pub use verifier::{extract_kid_from_jws, resolve_verification_key, verify_jws};
