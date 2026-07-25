//! The **enacted** EU repairability index — Regulation (EU) 2023/1669.
//!
//! This is the index displayed on the energy label for smartphones and slate
//! tablets, applicable since 2025-06-20. Its calculation method is Annex IV
//! point 5; its class boundaries are Annex II Table 4.
//!
//! ```text
//! R = SDD*0,25 + SF*0,15 + ST*0,15 + SSP*0,15 + SSU*0,15 + SRI*0,15
//! ```
//!
//! Every parameter is scored 1–5, so **R runs 1,00–5,00**. Class A is R ≥ 4,00,
//! class E bottoms out at 1,00.
//!
//! ⚠️ **Not the same thing as [`crate::repairability`].** That module is a
//! non-regulatory six-factor heuristic on a 0–10 scale. The two outputs are not
//! comparable and must never share a field or a column. This module produces a
//! regulatory class; that one produces a heuristic band.
//!
//! **What this module can and cannot do.** SDD, SF and ST derive from a
//! documented disassembly procedure and the technical documentation — data an
//! operator declares, not data we can independently verify. This calculates the
//! index and class from declared part-level scores and validates their ranges
//! and internal consistency. It does not verify the underlying step counts,
//! fastener types or tool requirements.
//!
//! ## Module layout
//!
//! - [`parameters`] — [`RepairabilityIndexInputs`], [`PriorityPartScores`].
//! - [`thresholds`] — the ruleset trait, weights, class boundaries, [`Eu2023_1669Ruleset`].
//! - [`calculator`] — [`calculate`], [`RepairabilityIndexResult`], [`RepairabilityClass`].

pub mod calculator;
pub mod parameters;
pub mod thresholds;

#[cfg(test)]
mod golden_vectors;

pub use calculator::{RepairabilityClass, RepairabilityIndexResult, calculate};
pub use parameters::{PriorityPartScores, RepairabilityIndexInputs};
pub use thresholds::{
    Eu2023_1669Ruleset, IndexClassBoundaries, IndexWeights, PartWeights, RepairabilityIndexRuleset,
};
