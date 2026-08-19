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
//! use dpp_calc::clock::AssessmentClock;
//! use dpp_calc::{repairability, ruleset_registry};
//!
//! // The date the law attached to this product — from its own record, never today.
//! let clock = AssessmentClock::placed_on(product.placed_on_market_date);
//!
//! match ruleset_registry::resolve_repairability("smartphone-tablet", clock.law_in_force_on) {
//!     Assessability::Assessed(ruleset) => {
//!         let result = repairability::calculate(&inputs, ruleset, clock)?;
//!     }
//!     // Each of these is a different thing to tell an operator, and none of
//!     // them is "non-compliant".
//!     Assessability::NotYetInForce { applies_from, .. } => { /* applies from … */ }
//!     Assessability::Undetermined { empowerment, .. } => { /* awaiting … */ }
//!     Assessability::Expired { superseded_by, .. } => { /* see successor … */ }
//!     Assessability::OutOfScope => { /* category not covered */ }
//! }
//! ```
//!
//! ## Adding a new version
//!
//! When `SimplifiedRepairabilityHeuristicV2` is introduced (e.g. from 2027-01-01):
//! 1. Close the current one: `Effectivity::closed(from, 2026-12-31)`.
//! 2. Add `SimplifiedRepairabilityHeuristicV2` with `from = 2027-01-01`.
//! 3. Add a second `"smartphone-tablet"` row to the table — no caller changes.
//!
//! ## Sunset policy
//!
//! Old rulesets stay in the table indefinitely. Their `Effectivity::InForce.until`
//! gates them from new calculations. Receipts reference rulesets by ID+version,
//! so removing a ruleset would make old receipts unverifiable. Never delete rows.

mod resolve;
mod status;

#[cfg(test)]
mod tests;

pub use resolve::{
    all_rulesets, resolve_recycled_content, resolve_repairability, resolve_repairability_index,
};
pub use status::{CalculatorImpl, CalculatorStatus, SectorCalculatorEntry, sector_calculator_map};
