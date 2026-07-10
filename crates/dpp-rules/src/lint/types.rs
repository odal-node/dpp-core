//! Shared lint finding types.

use alloc::string::String;

/// How strongly a lint finding should be read. Never blocking either way —
/// the distinction is tone, not gating (ADR-002: informative only).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LintSeverity {
    /// A data point that is very likely a mistake (e.g. a unit-conversion
    /// mismatch between two fields describing the same physical quantity).
    Warning,
    /// A softer signal — plausible but worth a second look (e.g. a claim
    /// with no supporting field, or a value near a physically wide bound).
    Notice,
}

/// A single plausibility finding. Phrased as a question, never a verdict —
/// callers should render `message` as-is.
#[derive(Debug, Clone, PartialEq)]
pub struct LintFinding {
    /// Stable machine-readable code, e.g. `"battery.energy_capacity_mismatch"`.
    pub code: &'static str,
    /// camelCase field locator this finding is primarily about.
    pub field: &'static str,
    pub severity: LintSeverity,
    /// Human-readable finding, phrased as a question.
    pub message: String,
}
