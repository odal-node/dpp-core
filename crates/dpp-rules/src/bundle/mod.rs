//! Compliance Current — signed, versioned ruleset bundles (open format).
//!
//! ADR-002's moat made literal: rulesets ship as versioned bundles whose
//! manifest is signed (compact EdDSA JWS) by an **offline publisher key**,
//! distinct from any operator key. A verifier pins the publisher public key,
//! checks fail-closed, and only then trusts the bundle. "Provably more
//! current than a fork" becomes a wire artifact a customer or auditor can
//! verify with an Apache-licensed library call, not a consulting promise.
//!
//! ## Bundle format
//!
//! A bundle is `{ manifestJws, content }` ([`SignedBundle`]):
//! - `manifestJws` — a compact EdDSA JWS whose payload is the
//!   [`RulesetManifest`] (bundle version, effective date, EU-act citations,
//!   sector schema versions, and the SHA-256 of `content`), signed by the
//!   publisher key.
//! - `content` — the ruleset payload the manifest commits to (thresholds,
//!   tables, schema references).
//!
//! ## What lives here vs. engine-side
//!
//! This module carries the **format types and fail-closed verification**
//! ([`verify_bundle`]) only. Signing (needs a private key store), hot-swap
//! runtime state, and reading bundle files from disk are engine concerns and
//! stay there. Verification itself doesn't depend on a JWS/crypto crate
//! directly — see [`JwsVerify`] — because `dpp-crypto` depends on
//! `dpp-domain`, which depends on this crate; a direct dependency the other
//! way would be a cycle. Callers inject their own EdDSA verifier (e.g. a thin
//! wrapper over `dpp-crypto`'s JWS verifier).

#[cfg(test)]
mod tests;
mod types;
mod verify;

pub use types::{RulesetError, RulesetManifest, SignedBundle, VerifiedRuleset};
pub use verify::{JwsVerify, content_hash, verify_bundle};
