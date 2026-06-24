//! `dpp-rules` — pure EU ESPR cross-field regulatory rules.
//!
//! Rules are grouped by sector module. Batteries, textiles, and electronics are
//! the active sectors; all others have placeholder modules and will be populated
//! in a later phase.
//!
//! Inputs are primitive borrowing views so each caller adapts its own
//! representation — typed structs in core, `serde_json::Value` fields in
//! plugins — without this crate depending on either.
//!
//! See `docs/architecture/SECTOR-MODEL-CONSOLIDATION.md` §7.

#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

#[cfg(test)]
extern crate std;

// Shared helpers (cross-sector utilities).
pub mod common;

// Chemical substance rules — REACH, RoHS, EU 2026/405.
// SVHC lives here rather than under any single sector because REACH Art. 33
// applies across textiles, electronics, toys, construction, and more.
pub mod chemicals;

// Active sectors.
pub mod batteries;
pub mod electronics;
pub mod textiles;

// Placeholder sectors — rules to be implemented in a later phase.
pub mod construction;
pub mod metals;
pub mod toys;

// ── Crate-root re-exports ────────────────────────────────────────────────────
// Preserved for backward compatibility with existing callers
// (dpp-domain adapters, dpp-plugin-sdk::rules).

pub use chemicals::cas::validate_cas_format;
pub use chemicals::surfactants::{
    SURFACTANT_BANDS, SurfactantInput, surfactant_band_valid, validate_surfactants,
};
pub use chemicals::svhc::{
    ECHA_CANDIDATE_LIST, SVHC_THRESHOLD_PCT, SvhcFinding, SvhcFindingKind, SvhcInput,
    check_svhc_declarations, validate_svhc_substances,
};
pub use common::country::country_code_valid;
pub use textiles::fibre::{
    FIBRE_SUM_TOLERANCE, FibreInput, fibre_sum_ok, validate_fibre_composition,
};
