//! Metals — CBAM (EU Regulation 2023/956) and EU ESPR sector rules.
//!
//! Steel and aluminium carry distinct production routes and CO₂e thresholds,
//! so each gets its own sub-module. Shared structural helpers (e.g. scrap-ratio
//! bounds) live here in mod.rs if and only if both sub-modules use them.
//!
//! Placeholder — rules to be implemented in a later phase.

pub mod aluminium;
pub mod steel;
