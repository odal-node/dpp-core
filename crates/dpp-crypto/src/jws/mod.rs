//! JWS signing primitives — EdDSA, ES256, ES384 and ES512 over RFC 8785 (JCS)
//! canonical bytes, each bound to the algorithm its key is recorded as using.

pub mod algorithm;
pub mod canonical;
mod error;
mod header_refusal;
pub mod public_key;
pub mod signer;
#[cfg(test)]
mod tests;
pub mod verifier;

pub use algorithm::{
    ED25519_ALG, ED25519_CRV, EDDSA_ALG, ES256_ALG, ES384_ALG, ES512_ALG,
    JWK_THUMBPRINT_URI_PREFIX, KeyAlgorithm, P256_CRV, P384_CRV, P521_CRV, is_allowed_alg,
};
pub use canonical::canonicalize;
pub use error::{PublicKeyError, SignatureLengthError};
pub use header_refusal::HeaderRefusal;
pub use public_key::PublicKey;
pub use signer::{sign, sign_typed, sign_typed_with, verify};
pub use verifier::{extract_kid_from_jws, header_refusal, resolve_verification_key, verify_jws};
