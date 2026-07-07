//! Threshold tables and product-category rulesets for the **simplified
//! repairability heuristic**.
//!
//! ⚠️ This is **not** the enacted EU 2023/1669 (smartphones/tablets) Annex IV
//! repairability index. That methodology uses a different parameter set
//! (incl. Fasteners & Tools), a 1–5 per-class scale, a priority-part dimension,
//! and its own class boundaries. The model here is a transparent six-factor
//! 0–2 heuristic; its output is a heuristic band, not a regulatory class.
//!
//! ## Module layout
//!
//! - `types` — the shared [`RepairabilityWeights`] / [`RepairabilityThresholds`] tables.
//! - `ruleset` — the [`RepairabilityRuleset`] trait.
//! - one file per concrete ruleset version: `smartphone` (in force today),
//!   `laptop`, `displays`, `washing_machine` (reserved stubs).

mod displays;
mod laptop;
mod ruleset;
mod smartphone;
#[cfg(test)]
mod tests;
mod types;
mod washing_machine;

pub use displays::DisplaysRuleset;
pub use laptop::LaptopRuleset;
pub use ruleset::RepairabilityRuleset;
pub use smartphone::SimplifiedRepairabilityHeuristic;
pub use types::{RepairabilityThresholds, RepairabilityWeights};
pub use washing_machine::WashingMachineRuleset;
