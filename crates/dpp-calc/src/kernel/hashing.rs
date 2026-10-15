//! JCS (RFC 8785) canonical hashing helpers for calculator inputs and outputs.
//!
//! These back the input/output hashes stored in a [`CalculationReceipt`], so an
//! auditor can confirm the same inputs deterministically produce the same outputs.
//!
//! The digest itself is [`dpp_rules::canonical::content_hash`], the one content
//! hasher this workspace has. A receipt's hashes are integrity hashes like any
//! other, and a second implementation would agree with the first only until one
//! of them changed.
//!
//! [`CalculationReceipt`]: super::receipt::CalculationReceipt

use super::error::CalcError;

/// SHA-256 of the JCS (RFC 8785) canonical JSON serialisation of `value`.
///
/// Fails if `value` cannot be serialized to JCS — returns the error rather
/// than silently hashing empty bytes.
pub fn jcs_hash<T: serde::Serialize>(value: &T) -> Result<String, CalcError> {
    dpp_rules::canonical::content_hash(value)
        .map_err(|e| CalcError::CanonicalizeError(e.to_string()))
}

/// SHA-256 of the JCS canonical JSON of calculator inputs — alias for [`jcs_hash`].
pub fn input_hash<T: serde::Serialize>(value: &T) -> Result<String, CalcError> {
    jcs_hash(value)
}
