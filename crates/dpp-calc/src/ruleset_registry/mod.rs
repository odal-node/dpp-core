//! Date-based ruleset resolution for all registered methodologies.
//!
//! Each `resolve_*` function answers: "given a product category and a date,
//! which ruleset version is legally in force?" Multiple versions of the same
//! product category can coexist in the table (sequential or overlapping date
//! ranges); the filter returns the one active on `on_date`.
//!
//! ## Module layout
//!
//! - `resolve` — [`all_rulesets`] + the `resolve_*` lookups.
//! - `status`  — the machine-readable [`CalculatorStatus`] map.
//!
//! ## Caller pattern
//!
//! ```rust,ignore
//! use chrono::Utc;
//! use dpp_calc::ruleset_registry;
//! use dpp_calc::repairability;
//!
//! let today = Utc::now().date_naive();
//! let ruleset = ruleset_registry::resolve_repairability("smartphone-tablet", today)
//!     .expect("no repairability ruleset in force for this category today");
//!
//! let result = repairability::calculate(&inputs, ruleset)?;
//! ```
//!
//! ## Adding a new version
//!
//! When `SimplifiedRepairabilityHeuristicV2` is introduced (e.g. from 2027-01-01):
//! 1. Set `SimplifiedRepairabilityHeuristic::effective_dates().until = Some(2026-12-31)`.
//! 2. Add `SimplifiedRepairabilityHeuristicV2` with `from = 2027-01-01`.
//! 3. Add a second `"smartphone-tablet"` row to the table — no caller changes.
//!
//! ## Sunset policy
//!
//! Old rulesets stay in the table indefinitely. Their `EffectiveDateBound.until`
//! gates them from new calculations. Receipts reference rulesets by ID+version,
//! so removing a ruleset would make old receipts unverifiable. Never delete rows.

mod resolve;
mod status;

#[cfg(test)]
mod tests;

pub use resolve::{all_rulesets, resolve_repairability};
pub use status::{CalculatorStatus, SectorCalculatorEntry, sector_calculator_map};
