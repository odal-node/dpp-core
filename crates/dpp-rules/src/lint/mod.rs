//! Passport plausibility lints — non-binding findings that flag *implausible*
//! data (arithmetic that doesn't add up, values outside physically plausible
//! ranges, or fields whose declared values are inconsistent with each other),
//! as distinct from the *malformed* data JSON Schema catches or the
//! *non-compliant* data a [`crate::batteries`]/[`crate::textiles`]-style rule
//! catches. A lint finding never blocks publish — see [`LintSeverity`].
//!
//! Each lint cites the physics or arithmetic behind it, the way the product group
//! rule modules cite the regulatory article behind a binding rule. Findings
//! are phrased as questions ("intended?"), never verdicts.

mod types;

pub mod battery;
pub mod svhc;
#[cfg(test)]
mod svhc_tests;
pub mod textile;
pub mod unsold_goods;

pub use types::{LintFinding, LintSeverity};

/// Version of this crate's lint pack. Bump whenever a lint is added, removed,
/// or its trigger condition changes — callers surface this alongside
/// findings so a consumer can tell which pack produced them.
///
/// 1.1.0 adds the three [`svhc`] lints. They are the first pack that is not
/// scoped to one product group: REACH Art. 33 reaches every article, so the
/// lints run wherever a payload declares substances rather than under a single
/// product group's arm.
pub const LINT_PACK_VERSION: &str = "1.1.0";
