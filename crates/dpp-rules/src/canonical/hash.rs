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
    // The canonicalizer runs first, so a non-finite float is refused here and the
    // Value below never holds one: it is only taken to inspect the integers.
    let canonical = serde_jcs::to_vec(value)?;
    reject_inexact_integers(&serde_json::to_value(value)?)?;
    Ok(hex::encode(Sha256::digest(canonical)))
}

/// Largest integer magnitude every JSON number model holds exactly (2^53 - 1).
const MAX_EXACT_INTEGER: u64 = (1 << 53) - 1;

/// The canonicalizer writes every integer as a double, so two integers beyond
/// 2^53 can share one digest. Refuse them rather than hash a value the caller
/// did not supply.
fn reject_inexact_integers(value: &serde_json::Value) -> Result<(), serde_json::Error> {
    match value {
        serde_json::Value::Number(n) => {
            let magnitude = n.as_u64().or_else(|| n.as_i64().map(i64::unsigned_abs));
            if magnitude.is_some_and(|m| m > MAX_EXACT_INTEGER) {
                return Err(serde::ser::Error::custom(format!(
                    "integer {n} is outside ±(2^53 - 1) and cannot be hashed exactly"
                )));
            }
            Ok(())
        }
        serde_json::Value::Array(items) => items.iter().try_for_each(reject_inexact_integers),
        serde_json::Value::Object(map) => map.values().try_for_each(reject_inexact_integers),
        serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::String(_) => {
            Ok(())
        }
    }
}
