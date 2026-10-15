use sha2::{Digest, Sha256};

/// Canonical SHA-256 (hex) of a JSON value, over its RFC 8785 (JCS) bytes.
///
/// Because canonicalisation is order-independent, a signer and a verifier
/// reach the same digest for semantically identical values — that is the
/// property every caller of this function depends on.
///
/// Takes any `Serialize`, not only a `serde_json::Value`, so a caller holding a
/// typed value never has to route it through `serde_json::to_value` first. That
/// route is not neutral: it turns a non-finite float into `null`, so the hash
/// would cover a value the caller never had instead of refusing the one it has.
///
/// **Fallible by design, and it must stay that way.** RFC 8785 rejects
/// non-finite floats (`NaN`/`Infinity`). A caller that "knows" its own input is
/// finite still must not collapse this into a panic: the point of a tamper hash
/// is that untrusted input reaches it, and a hasher that aborts the process on
/// hostile input has traded an error path for a denial of service.
///
/// # Errors
/// Returns the underlying serialisation error if `value` cannot be
/// JCS-canonicalised.
pub fn content_hash<T: serde::Serialize + ?Sized>(value: &T) -> Result<String, serde_json::Error> {
    Ok(hex::encode(Sha256::digest(serde_jcs::to_vec(value)?)))
}
