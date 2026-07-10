//! JSON Schema + cross-field validation for sector-specific DPP data.
//!
//! The schema step resolves against the crate's **embedded**
//! [`VersionedSchemaRegistry`](crate::schemas::VersionedSchemaRegistry) — built
//! once from the compile-time schemas — at the version the
//! [`SectorCatalog`](crate::SectorCatalog) marks current for the sector; there
//! are no per-sector validators and no hardcoded versions here. Schemas
//! registered at runtime into a separate registry instance are **not** seen by
//! these free functions (nor by `Passport::validate`); validate against those
//! through that registry directly (its fail-closed `validate_strict`).
//! Cross-field regulatory rules (which JSON Schema cannot express, e.g. "fibre
//! percentages sum to ~100%") come from `dpp-rules` via the `dpp-domain` adapters.
//! See `docs/architecture/SECTOR-MODEL-CONSOLIDATION.md` (step C2).
//!
//! **Note**: excluded from wasm32 builds since jsonschema depends on reqwest's
//! blocking API.
//!
//! ## Module layout
//!
//! - [`validator`] — the [`SectorValidator`] trait + [`SectorValidatorRegistry`]
//!   extensibility seam (a port-like abstraction, different change-cadence).
//! - [`functions`] — the `validate_*` free functions (schema + cross-field).
//! - [`batch`] — batch validation over multiple sector-data items.

#![cfg(not(target_arch = "wasm32"))]

pub mod batch;
pub mod functions;
pub mod validator;

#[cfg(test)]
mod tests;

pub use batch::{BatchValidationItem, batch_errors, validate_sector_data_batch};
pub use functions::{
    validate_raw_sector_data, validate_sector_data, validate_sector_data_with_registry,
};
pub use validator::{SectorValidator, SectorValidatorRegistry};

// `FieldError` and `ValidationErrors` live in `crate::domain::field_error`
// (wasm-safe) so `DppError` can carry structured validation detail.
