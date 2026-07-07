//! Simplified repairability heuristic — six-parameter, 0–2 each, A–E band.
//!
//! ⚠️ **Non-regulatory.** This is a transparent six-factor indicator
//! (disassembly, spare parts, repair info, diagnostic tools, software updates,
//! customer support), each scored 0–2. It is **not** the enacted EU 2023/1669
//! (smartphones/tablets) Annex IV repairability index, which uses a different
//! parameter set (incl. Fasteners & Tools), a 1–5 per-class scale, a
//! priority-part dimension, and its own class boundaries. The A–E result is a
//! heuristic band and must not be presented as a regulatory repairability class.
//! A faithful EU 2023/1669 Annex IV implementation is tracked separately.
//!
//! **Usage:**
//! ```rust
//! use dpp_calc::repairability::{
//!     calculate, parameters::RepairabilityInputs,
//!     thresholds::SimplifiedRepairabilityHeuristic,
//! };
//!
//! let inputs = RepairabilityInputs {
//!     disassembly: 2,
//!     spare_parts: 2,
//!     repair_info: 2,
//!     diagnostic_tools: 1,
//!     software_updatability: 2,
//!     customer_support: 1,
//! };
//! let result = calculate(&inputs, &SimplifiedRepairabilityHeuristic).unwrap();
//! println!("{:?}  ({:.2}/10)", result.class, result.numeric_score);
//! ```
//!
//! ## Module layout
//!
//! - [`calculator`] — [`calculate`], [`RepairabilityResult`], [`RepairabilityClass`].
//! - [`parameters`] — [`RepairabilityInputs`].
//! - [`thresholds`] — the [`thresholds::RepairabilityRuleset`] trait + concrete rulesets.
//! - `golden_vectors` — worked-example regression tests.

pub mod calculator;
pub mod parameters;
pub mod thresholds;

#[cfg(test)]
mod golden_vectors;

pub use calculator::{ParameterContributions, RepairabilityClass, RepairabilityResult, calculate};
pub use parameters::RepairabilityInputs;
pub use thresholds::{
    DisplaysRuleset, LaptopRuleset, RepairabilityRuleset, SimplifiedRepairabilityHeuristic,
    WashingMachineRuleset,
};
