//! RFC 8785 JSON Canonicalization Scheme (JCS).
//!
//! Defines the **one** canonical byte form of a JSON value. This is the signing
//! contract: [`crate::jws::signer`] signs over these bytes, and every content-binding
//! verifier (`dpp_vc::local_service`) recomputes them and compares.
//! Signature equivalence therefore rests on a defined canonical form — not on
//! incidental serde behaviour (object key order, number formatting, unicode
//! escaping) that can differ between serializers and silently break
//! content-binding.
//!
//! Backed by `serde_jcs` (RFC 8785). Kept behind this single function so the
//! canonicalization implementation can be swapped without touching callers.

use serde_json::Value;

/// Largest integer magnitude every JSON number model holds exactly: 2^53 - 1,
/// the bound that RFC 8785 takes from the I-JSON profile of JSON.
const MAX_EXACT_INTEGER: u64 = (1 << 53) - 1;

/// Canonicalize a JSON value to its RFC 8785 (JCS) byte form.
///
/// Guarantees: object keys sorted by UTF-16 code unit, no insignificant
/// whitespace, numbers in the ECMAScript `Number::toString` form (so `1.0`
/// serializes as `1`), and consistent string escaping.
///
/// Integers beyond ±(2^53 − 1) are refused. The serializer writes every integer
/// as an IEEE 754 double, so two different integers can share one canonical form
/// and a signature over one would verify against the other.
pub fn canonicalize(value: &Value) -> anyhow::Result<Vec<u8>> {
    reject_inexact_integers(value)?;
    serde_jcs::to_vec(value).map_err(|e| anyhow::anyhow!("JCS canonicalization: {e}"))
}

fn reject_inexact_integers(value: &Value) -> anyhow::Result<()> {
    match value {
        Value::Number(n) => {
            // Only an integer is checked. A number `serde_json` holds as a double
            // (`1e21`, or `9007199254740993.0`, which it reads as 2^53) is written
            // as that same double, so the canonical form is the value the caller
            // holds, and RFC 8785 Appendix B prints such doubles.
            let magnitude = n.as_u64().or_else(|| n.as_i64().map(i64::unsigned_abs));
            anyhow::ensure!(
                magnitude.is_none_or(|m| m <= MAX_EXACT_INTEGER),
                "integer {n} is outside ±(2^53 - 1) and cannot be canonicalized exactly"
            );
            Ok(())
        }
        Value::Array(items) => items.iter().try_for_each(reject_inexact_integers),
        Value::Object(map) => map.values().try_for_each(reject_inexact_integers),
        Value::Null | Value::Bool(_) | Value::String(_) => Ok(()),
    }
}

#[cfg(test)]
mod tests;
