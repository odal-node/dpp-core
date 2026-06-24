//! JCS (RFC 8785) canonical hashing helpers for calculator inputs and outputs.
//!
//! These back the input/output hashes stored in a [`CalculationReceipt`], so an
//! auditor can confirm the same inputs deterministically produce the same outputs.
//!
//! [`CalculationReceipt`]: super::receipt::CalculationReceipt

use super::error::CalcError;

/// SHA-256 of the JCS (RFC 8785) canonical JSON serialisation of `value`.
///
/// Fails if `value` cannot be serialized to JCS — returns the error rather
/// than silently hashing empty bytes.
pub fn jcs_hash<T: serde::Serialize>(value: &T) -> Result<String, CalcError> {
    use sha2::{Digest, Sha256};
    let bytes =
        serde_jcs::to_vec(value).map_err(|e| CalcError::CanonicalizeError(e.to_string()))?;
    Ok(hex::encode(Sha256::digest(&bytes)))
}

/// SHA-256 of the JCS canonical JSON of calculator inputs — alias for [`jcs_hash`].
pub fn input_hash<T: serde::Serialize>(value: &T) -> Result<String, CalcError> {
    jcs_hash(value)
}
