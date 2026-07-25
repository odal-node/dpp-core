//! Open, data-driven catalog of EU ESPR sectors.
//!
//! The catalog is the single source of truth for *what sectors exist* and
//! *where each stands in the EU regulatory pipeline*. Unlike a closed `enum`,
//! sectors are described by **data** — one embedded manifest per sector at
//! `dpp-core/crates/dpp-domain/sectors/{key}.json` — and new sectors can be
//! added at runtime via [`SectorCatalog::register`] without recompiling core.
//!
//! Each [`SectorDescriptor`] ties together a sector's canonical key, regulatory
//! status, legal basis, schema versions, retention, product categories, and
//! plugin binding — resolving the "four spellings of a sector" problem by
//! giving every component one record to agree on.
//!
//! [`RegulatoryStatus`] gates behaviour: only `InForce` sectors may carry a
//! binding compliance determination. `Provisional` sectors (on the ESPR working
//! plan but without an adopted delegated act) are present but **flagged** —
//! their schemas are best-effort drafts and plugins must not assert
//! COMPLIANT/NON_COMPLIANT.
//!
//! ## Module layout
//!
//! - [`regime`] — the [`Regime`] legal-instrument axis.
//! - [`status`] — the [`RegulatoryStatus`] gate.
//! - [`descriptor`] — the [`SectorDescriptor`] record shape.
//! - [`error`] — [`CatalogError`].
//! - [`catalog`] — [`SectorCatalog`] itself, plus the embedded manifests.

#[allow(clippy::module_inception)]
pub mod catalog;
pub mod descriptor;
pub mod error;
pub mod regime;
pub mod status;

#[cfg(test)]
mod tests;

pub use catalog::SectorCatalog;
pub use descriptor::SectorDescriptor;
pub use error::CatalogError;
pub use regime::Regime;
pub use status::RegulatoryStatus;
