//! Open, data-driven catalogs of EU product groups and the acts that reach them.
//!
//! Two catalogs, deliberately separate, because they answer different questions
//! and change for different reasons.
//!
//! ## The product-group catalog
//!
//! [`SectorCatalog`] is the single source of truth for *what product groups
//! exist*. Unlike a closed `enum` they are described by **data** — one embedded
//! manifest per group at `crates/dpp-domain/sectors/{key}.json` — and new ones
//! can be added at runtime via [`SectorCatalog::register`] without recompiling.
//!
//! Each [`SectorDescriptor`] ties together a group's canonical key, schema
//! versions, product categories, disclosure classes and plugin binding —
//! resolving the "four spellings of a sector" problem by giving every component
//! one record to agree on. It carries **no law**; see its own docs for why.
//!
//! ## Module layout
//!
//! - [`descriptor`] — the [`SectorDescriptor`] record shape.
//! - [`error`] — [`CatalogError`].
//! - [`catalog`] — [`SectorCatalog`] itself, plus the embedded manifests.
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
//! It exists because a sector's law does not fit on the sector. ESPR Art. 5(7)
//! lets acts overlap with no precedence rule between them, so the applicable set
//! is a set; and an act may reach a product group we hold no manifest for, so
//! the reach has to be recorded on the act. `SectorDescriptor`'s singular
//! `regime`, `status`, `dppAppliesFrom` and `retentionYears` each assume one act
//! governs one sector, which is what this catalog stops assuming.
//!
//! **Not yet wired.** [`SectorCatalog`] remains the record every component
//! resolves against. Where the two disagree, the divergence is pinned by test so
//! a new one fails rather than accumulating quietly.
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
#[allow(clippy::module_inception)]
pub mod catalog;
pub mod descriptor;
pub mod error;
pub mod granularity;
pub mod instrument;
pub mod instrument_catalog;
pub mod instrument_kind;
pub mod instrument_status;
pub mod passport_obligation;

pub mod retention;
pub mod status;

#[cfg(test)]
mod instrument_tests;
#[cfg(test)]
mod tests;

pub use binding::InstrumentBinding;
pub use catalog::SectorCatalog;
pub use descriptor::SectorDescriptor;
pub use error::CatalogError;
pub use granularity::Granularity;
pub use instrument::Instrument;
pub use instrument_catalog::InstrumentCatalog;
pub use instrument_kind::InstrumentKind;
pub use instrument_status::InstrumentStatus;
pub use passport_obligation::{DateBasis, ObligationDate, PassportObligation};

pub use retention::RetentionBasis;
pub use status::RegulatoryStatus;
