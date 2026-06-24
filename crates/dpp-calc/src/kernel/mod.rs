//! Methodology-agnostic spine shared by every calculator.
//!
//! These types are the contracts a calculator builds on, independent of any
//! specific EU methodology:
//!
//! - [`error`]   — the single [`CalcError`](error::CalcError) error type.
//! - [`ruleset`] — the [`Ruleset`](ruleset::Ruleset) framework: identity,
//!   validity period, and structured legal citation.
//! - [`receipt`] — the proof-of-calculation envelope plus JCS hashing helpers.
//! - [`factor`]  — the runtime injection point for licensed LCI factor datasets.
//!
//! The spine has **no dependency on any methodology** (`co2e`, `repairability`).
//! Methodologies depend on the spine, never the reverse. The crate root
//! re-exports these modules under their original names (`dpp_calc::error`,
//! `dpp_calc::ruleset`, …) so the grouping is an internal detail.

pub mod error;
pub mod factor;
pub mod hashing;
pub mod receipt;
pub mod ruleset;

#[cfg(any(test, feature = "synthetic-factors"))]
mod synthetic_factor;

#[cfg(test)]
mod tests;
