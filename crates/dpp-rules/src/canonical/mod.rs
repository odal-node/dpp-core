//! Canonical JSON hashing (RFC 8785 / JCS) — the single content hasher.
//!
//! Deliberately shared: integrity-hash provenance for the signed ruleset
//! channel ([`crate::bundle`]) and for a consumer's evidence dossier must be
//! the *same* function, not two implementations that agree today and drift
//! silently later. This crate is the deepest in the dependency graph
//! (`dpp-crypto` → `dpp-domain` → `dpp-rules`), so it is the only place a
//! shared implementation can live without creating a cycle.
//!
//! Compiled under the `bundle` feature, which is what pulls in the JCS and
//! SHA-256 dependencies. Splitting a narrower `canonical` feature out of
//! `bundle` is a one-line change if a consumer ever needs the hasher without
//! the bundle format; nothing does today.

mod hash;
#[cfg(test)]
mod tests;

pub use hash::content_hash;
