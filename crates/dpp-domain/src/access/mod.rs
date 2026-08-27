//! ProductGroup access policy — the per-field disclosure contract and the filter that
//! applies it.
//!
//! Which fields a role may see describes what a passport *is*: the classes are
//! declared as data in the product group manifests, and the contract is part of what
//! the standard promises third parties.
//!
//! Distinct from `dpp-vc`, which establishes *which* audience a caller holds. A
//! credential proves the role; this maps the role to fields.

#[cfg(test)]
mod classification_tests;
pub mod filter;
#[cfg(test)]
mod lattice_tests;
pub mod passport_view;
#[cfg(test)]
mod passport_view_tests;
#[cfg(test)]
mod path_matching_tests;
pub mod policy;
#[cfg(test)]
mod policy_tests;
#[cfg(test)]
mod redaction_tests;
#[cfg(test)]
mod ref_path_tests;
#[cfg(test)]
mod ref_walk_tests;
#[cfg(test)]
mod schema_class_resolution_tests;
#[cfg(test)]
mod tests;

pub use filter::{PolicyDecision, filter_by_audience, filter_by_audience_in_scope};
pub use passport_view::redact_passport;
pub use policy::{DocumentScope, ProductGroupAccessPolicy};
