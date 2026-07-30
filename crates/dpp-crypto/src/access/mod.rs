//! Sector access policy — the per-field disclosure contract and the filter that
//! applies it.
//!
//! ⬅️ **Settled home is `dpp-domain`.** Which fields a role may see describes
//! what a passport *is*: the classes are declared as data in the sector
//! manifests, and the contract is part of what the standard promises third
//! parties. It stays here only until `dpp-domain` is analysed on its own terms
//! — moving it into `dpp-vc` en route would be a move twice. See
//! `dpp-docs/decisions/ADR-010-crate-architecture.md`.
//!
//! Distinct from `dpp-vc`, which establishes *which* audience a caller holds. A
//! credential proves the role; this maps the role to fields.

pub mod filter;
pub mod policy;
#[cfg(test)]
mod tests;

pub use filter::{PolicyDecision, filter_by_audience};
pub use policy::SectorAccessPolicy;
