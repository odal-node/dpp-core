//! Lineage rules — the cross-field checks that bind one passport to another.
//!
//! Top-level rather than under a product group because the edges are envelope
//! fields every product group carries, and because the linkage obligation is
//! framework-level: **ESPR (EU) 2024/1781 Art. 11(d)** requires that "where a
//! new digital product passport is created for a product that already has a
//! digital product passport, the new digital product passport shall be linked to
//! the original digital product passport or passports."
//!
//! The *operation vocabulary* those edges use is battery-specific
//! (Reg. (EU) 2023/1542 Art. 77(7)), but the shape of the rule is not, and
//! filing it under `batteries` would imply only batteries can have second-life
//! lineage — which the envelope contradicts.

pub mod consent;
pub mod consistency;
pub mod finding;
pub mod input;
pub mod status_defect;

#[cfg(test)]
mod consent_tests;
#[cfg(test)]
mod consistency_tests;

pub use consent::check_derivation_consent;
pub use consistency::check_life_status_consistency;
pub use finding::{ConsentDefect, ConsentFinding};
pub use input::{DerivationEdge, TransferEvidence};
pub use status_defect::StatusDefect;
