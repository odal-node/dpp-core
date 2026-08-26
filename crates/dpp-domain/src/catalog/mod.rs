//! The open, data-driven catalog of EU product groups.
//!
//! This is one of two catalogs and it carries **no law**. The acts, their
//! bindings and the obligations they create live in
//! [`crate::instrument`], which was lifted out of here because it is a
//! different axis that changes for different reasons.
//!
//! [`ProductGroupCatalog`] is the single source of truth for *what product groups
//! exist*. Unlike a closed `enum` they are described by **data** — one embedded
//! manifest per group at `crates/dpp-domain/product-groups/{key}.json` — and new ones
//! can be added at runtime via [`ProductGroupCatalog::register`] without recompiling.
//!
//! Each [`ProductGroupDescriptor`] ties together a group's canonical key, schema
//! versions, product categories, disclosure classes and plugin binding —
//! resolving the "four spellings of a product group" problem by giving every component
//! one record to agree on. It carries **no law**; see its own docs for why.
//!
//! ## Module layout
//!
//! - [`descriptor`] — the [`ProductGroupDescriptor`] record shape.
//! - [`error`] — [`CatalogError`].
//! - [`granularity`] — the [`Granularity`] level a passport describes.
//! - [`product_group`] — [`ProductGroupCatalog`] itself, plus the embedded manifests.
//! - [`retention`] — the [`RetentionBasis`] provenance marker.
//! - [`status`] — the [`RegulatoryStatus`] determination gate, per binding.
//!
//! ## Where the law is, and is not
//!
//! Not here. A product group's law does not fit on the product group: ESPR Art.
//! 5(7) lets acts overlap with no precedence rule between them, so the
//! applicable set is a *set*; and an act may reach a group no manifest models,
//! so the reach has to be recorded on the act rather than the group.
//!
//! [`ProductGroupDescriptor`] used to carry a singular `regime`, `status`,
//! `dppAppliesFrom` and `retentionYears`, each assuming one act governs one
//! group. Those fields are gone. The acts, their bindings and the obligations
//! they create live in [`crate::instrument`], and the two catalogs no
//! longer overlap and so cannot disagree: this one answers *what groups exist
//! and how we serve them*, that one answers *what binds them*. Anything asking
//! whether an obligation applies goes there.
pub mod descriptor;
pub mod error;
pub mod granularity;
pub mod product_group;
pub mod retention;
pub mod status;

#[cfg(test)]
mod parity_tests;
#[cfg(test)]
mod tests;

pub use descriptor::ProductGroupDescriptor;
pub use error::CatalogError;
pub use granularity::Granularity;
pub use product_group::ProductGroupCatalog;
pub use retention::RetentionBasis;
pub use status::RegulatoryStatus;
