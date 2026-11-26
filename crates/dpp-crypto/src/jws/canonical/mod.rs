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

mod jcs;
#[cfg(test)]
mod tests;

pub use jcs::*;
