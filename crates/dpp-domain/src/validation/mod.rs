//! JSON Schema + cross-field validation for product group-specific DPP data.
//!
//! The schema step resolves against the crate's **embedded**
//! [`VersionedSchemaRegistry`](crate::schemas::VersionedSchemaRegistry) — built
//! once from the compile-time schemas — at the version the
//! [`ProductGroupCatalog`](crate::ProductGroupCatalog) marks current for the product group; there
//! are no per-product group validators and no hardcoded versions here. Schemas
//! registered at runtime into a separate registry instance are **not** seen by
//! these free functions (nor by `Passport::validate`); validate against those
//! through that registry directly (its fail-closed `validate_strict`).
//! Cross-field regulatory rules (which JSON Schema cannot express, e.g. "fibre
//! percentages sum to ~100%") come from `dpp-rules` via the `dpp-domain` adapters.
//!
//! **Note**: excluded from wasm32 builds since jsonschema depends on reqwest's
//! blocking API.
//!
//! ## Module layout
//!
//! - [`validator`] — the [`ProductGroupValidator`] trait + [`ProductGroupValidatorRegistry`]
//!   extensibility seam (a port-like abstraction, different change-cadence).
//! - [`functions`] — the `validate_*` free functions (schema + cross-field).
//! - [`batch`] — batch validation over multiple product group-data items.

// Only the schema pass needs gating. `rules` is a thin adapter onto `dpp-rules`
// and is pure, so it stays available on every target — the module-level gate
// that used to sit here withheld it from wasm32 for no reason of its own.
#[cfg(not(target_arch = "wasm32"))]
pub mod batch;
#[cfg(not(target_arch = "wasm32"))]
pub mod functions;
pub mod rules;
#[cfg(not(target_arch = "wasm32"))]
pub mod validator;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;

#[cfg(not(target_arch = "wasm32"))]
pub use batch::{BatchValidationItem, batch_errors, validate_product_group_data_batch};
#[cfg(not(target_arch = "wasm32"))]
pub use functions::{
    validate_passport, validate_product_group_data, validate_product_group_data_with_registry,
    validate_raw_product_group_data,
};
pub use rules::{
    battery_recycled_chemistry_conflicts, unsold_goods_annex_vii_heading,
    unsold_goods_cn_depth_is_correct, validate_battery_operating_temp, validate_fibre_composition,
    validate_surfactants, validate_svhc_substances,
};
#[cfg(not(target_arch = "wasm32"))]
pub use validator::{ProductGroupValidator, ProductGroupValidatorRegistry};

// `FieldError` and `ValidationErrors` live in `crate::error::field`
// (wasm-safe) so `DppError` can carry structured validation detail.
