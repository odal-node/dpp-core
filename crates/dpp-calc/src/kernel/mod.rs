//! Methodology-agnostic spine shared by every calculator.
//!
//! These types are the contracts a calculator builds on, independent of any
//! specific EU methodology:
//!
//! - [`error`]   — the single [`CalcError`](error::CalcError) error type.
//! - [`clock`]   — [`AssessmentClock`](clock::AssessmentClock): the governing-law
//!   date and the computation date, kept apart.
//! - [`assessability`] — [`Assessability`](assessability::Assessability): why a
//!   metric has a value, or which of four reasons it does not.
//! - [`ruleset`] — the [`Ruleset`](ruleset::Ruleset) framework: identity,
//!   effectivity, and structured legal citation.
//! - [`parameters`] — the numbers a ruleset carries, and the fill-never-override
//!   rule a signed bundle is held to when it offers replacements.
//! - [`receipt`] — the proof-of-calculation envelope plus JCS hashing helpers.
//! - [`factor`]  — the runtime injection point for licensed LCI factor datasets.
//!
//! The spine has **no dependency on any methodology** (`co2e`, `repairability`).
//! Methodologies depend on the spine, never the reverse. The crate root
//! re-exports these modules under their original names (`dpp_calc::error`,
//! `dpp_calc::ruleset`, …) so the grouping is an internal detail.

pub mod assessability;
pub mod clock;
pub mod error;
pub mod factor;
pub mod hashing;
pub mod parameters;
pub mod receipt;
pub mod ruleset;

#[cfg(any(test, feature = "synthetic-factors"))]
mod synthetic_factor;

#[cfg(test)]
mod tests;
