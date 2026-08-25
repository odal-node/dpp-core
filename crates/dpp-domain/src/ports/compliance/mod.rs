//! Open-core compliance boundary — strategy and registry traits.
//!
//! This module defines the extension seam used by proprietary compliance tiers.
//!
//! The open-source (Apache-2.0) binary wires `PassthroughRegistry`, which stores
//! manufacturer-supplied values verbatim without computing any scores.
//!
//! A proprietary binary can wire its own `PremiumComplianceRegistry`
//! implementation in a separate Cargo workspace without forking this crate.
//!
//!
//! The value objects these traits produce — `ComplianceResult` and friends —
//! are domain values, not ports, and live in [`crate::domain::compliance`].
//! They are **not** re-exported here: a second path to the same type is what
//! let a tier-2 aggregate import tier 4 in the first place (CODE-LAYOUT.md §1).

mod registry;
mod strategy;

pub use registry::ComplianceRegistry;
pub use strategy::ComplianceStrategy;
