//! Audit trail wire type and hash-chain verification.
//!
//! Promoted from `dpp-engine`'s `dpp-types::audit` (2026-07-07) — that
//! module's own doc comment flagged this shape as a "core-candidate": the
//! hash-chain format is third-party-verifiable, making it part of the
//! proof-bound standard rather than engine plumbing. `dpp-types::audit` now
//! re-exports [`AuditEntry`] from here and keeps only what is legitimately
//! engine-side: the `AuditRepository` persistence port and the
//! `AuthContext`-aware constructor convenience.

mod entry;
mod verify;

#[cfg(test)]
mod tests;

pub use entry::{AuditEntry, GENESIS_PREV_HASH};
pub use verify::{AuditChainBreak, verify_audit_chain};
