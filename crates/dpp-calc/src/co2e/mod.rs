//! Cradle-to-gate CO₂e calculator and Battery CFB stub.
//!
//! The primary entry point is [`calculate`], which computes the production-stage
//! footprint from a bill of materials and manufacturing energy using a
//! caller-supplied [`Co2eRuleset`]. [`cfb`] is a stub gated on Phase 2 data licensing.
//!
//! ## Module layout (five-file methodology convention)
//!
//! - [`parameters`] — typed inputs ([`Co2eInputs`], [`MaterialFootprint`]).
//! - [`thresholds`] — the [`Co2eRuleset`] trait and [`CradleToGateRuleset`] impl.
//! - [`calculator`] — the [`calculate`] algorithm and its output types.
//! - `golden_vectors` — `#[cfg(test)]` regression tests.
//! - [`cfb`] — battery CFB stub; [`gwp_factors`] — embeddable GWP100 table.

pub mod calculator;
pub mod cfb;
pub mod gwp_factors;
pub mod parameters;
pub mod thresholds;

#[cfg(test)]
mod golden_vectors;

pub use calculator::{Co2eResult, LifecycleStage, MaterialLineResult, calculate};
pub use parameters::{Co2eInputs, MaterialFootprint};
pub use thresholds::{Co2eRuleset, CradleToGateRuleset};
