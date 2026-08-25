//! Open, data-driven catalogs of EU product groups and the acts that reach them.
//!
//! Two catalogs, deliberately separate, because they answer different questions
//! and change for different reasons.
//!
//! ## The product-group catalog
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
//! - [`product_group`] — [`ProductGroupCatalog`] itself, plus the embedded manifests.
//! - [`status`] — the [`RegulatoryStatus`] determination gate, now per binding.
//! - [`retention`] — the [`RetentionBasis`] provenance marker.
//!
//! ## The instrument catalog
//!
//! A second catalog describes the **legal acts** that reach product groups:
//! [`Instrument`], its [`InstrumentKind`] and [`InstrumentStatus`], the
//! [`PassportObligation`] it imposes or does not, the [`Granularity`] it fixes,
//! and one [`InstrumentBinding`] per product group it reaches.
//!
//! It exists because a product group's law does not fit on the product group. ESPR Art. 5(7)
//! lets acts overlap with no precedence rule between them, so the applicable set
//! is a set; and an act may reach a product group we hold no manifest for, so
//! the reach has to be recorded on the act. `ProductGroupDescriptor` used to
//! carry a singular `regime`, `status`, `dppAppliesFrom` and `retentionYears`,
//! each of which assumes one act governs one product group — which is what this
//! catalog stops assuming.
//!
//! **This is where the law lives.** Those fields are gone from
//! [`ProductGroupDescriptor`], so the two catalogs no longer overlap and cannot
//! disagree: the product-group catalog answers *what groups exist and how we
//! serve them*, this one answers *what binds them*. Anything asking whether an
//! obligation applies must come here.
//!
//! - [`granularity`] — [`Granularity`], the model/batch/item level an act fixes.
//! - [`instrument_kind`] — [`InstrumentKind`], *what kind* of act.
//! - [`instrument_status`] — [`InstrumentStatus`], how far through the process.
//! - [`passport_obligation`] — [`PassportObligation`], [`ObligationDate`],
//!   [`DateBasis`].
//! - [`binding`] — [`InstrumentBinding`], one act's reach into one group.
//! - [`instrument`] — [`Instrument`], the act itself.
//! - [`instrument_catalog`] — [`InstrumentCatalog`] and its embedded manifests.

pub mod binding;

pub mod descriptor;
pub mod error;
pub mod granularity;
pub mod instrument;
pub mod instrument_catalog;
pub mod instrument_kind;
pub mod instrument_ref;
pub mod instrument_status;
pub mod obligation_date;
pub mod passport_obligation;
pub mod product_group;

pub mod retention;
pub mod status;

#[cfg(test)]
mod instrument_axis_tests;
#[cfg(test)]
mod instrument_kind_tests;
#[cfg(test)]
mod instrument_ref_tests;
#[cfg(test)]
mod instrument_tests;
#[cfg(test)]
mod parity_tests;
#[cfg(test)]
mod passport_obligation_tests;
#[cfg(test)]
mod tests;

pub use binding::InstrumentBinding;
pub use descriptor::ProductGroupDescriptor;
pub use error::CatalogError;
pub use granularity::Granularity;
pub use instrument::Instrument;
pub use instrument_catalog::InstrumentCatalog;
pub use instrument_kind::InstrumentKind;
pub use instrument_ref::{InstrumentRef, RecordedBasis};
pub use instrument_status::InstrumentStatus;
pub use obligation_date::{DateBasis, ObligationDate};
pub use passport_obligation::PassportObligation;
pub use product_group::ProductGroupCatalog;

pub use retention::RetentionBasis;
pub use status::RegulatoryStatus;
