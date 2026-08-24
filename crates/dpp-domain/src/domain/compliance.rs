//! Compliance determination value objects — the result of running a sector's
//! rules against a passport, and the errors that stop one being produced.
//!
//! These are **persisted domain values**, not ports: they are serialised onto
//! `Passport::compliance_result` and travel on the wire. The traits that
//! produce them are the extension seam and live in
//! [`crate::ports::compliance`].

use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ─── Output types ─────────────────────────────────────────────────────────

/// A single compliance finding (one rule outcome) attached to a determination.
///
/// Findings are split into [`ComplianceResult::violations`] (binding — block
/// publish for an in-force sector) and [`ComplianceResult::warnings`]
/// (advisory/experimental — never block). The vec a finding lands in encodes its
/// severity, so there is no separate severity field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComplianceFinding {
    /// Stable machine-readable code, e.g. `"battery.recycled_content.cobalt_below_2031"`.
    pub code: String,
    /// JSON-pointer-style field locator (e.g. `"/recycledContentCobaltPct"`), or
    /// empty when the finding is not tied to a single field.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub field: String,
    /// Human-readable explanation.
    pub message: String,
}

impl ComplianceFinding {
    /// Construct a finding from its code, field locator, and message.
    pub fn new(
        code: impl Into<String>,
        field: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            field: field.into(),
            message: message.into(),
        }
    }
}

/// Result of compliance calculation for a single product.
///
/// Carries computed metrics, an overall (regulatory-date-gated) status, and the
/// individual findings split into binding `violations` and advisory `warnings`.
/// A `ruleset_version` + `assessed_at` + `receipt` are populated when a
/// `dpp-calc` calculation actually ran.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComplianceResult {
    /// Calculated or manufacturer-supplied CO₂e score in kg.
    pub co2e_score: Option<f64>,
    /// Calculated or manufacturer-supplied repairability index (0.0–10.0).
    pub repairability_index: Option<f64>,
    /// Calculated or manufacturer-supplied recycled content percentage.
    pub recycled_content_pct: Option<f64>,
    /// Overall compliance determination.
    pub compliance_status: ComplianceStatus,
    /// Binding findings — block publish when the sector is in force. Empty for
    /// passthrough / not-assessed determinations.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub violations: Vec<ComplianceFinding>,
    /// Advisory / experimental findings — surfaced but never block publish (e.g.
    /// recycled-content thresholds that are not yet in force).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<ComplianceFinding>,
    /// Identifier + version of the resolved `dpp-calc` ruleset, when one ran.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ruleset_version: Option<String>,
    /// When the determination was computed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assessed_at: Option<DateTime<Utc>>,
    /// Serialized `dpp-calc` `CalculationReceipt` (input hash, ruleset id +
    /// version, factor dataset version + table hash) for notified-body audit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt: Option<serde_json::Value>,
}

impl Default for ComplianceResult {
    fn default() -> Self {
        Self {
            co2e_score: None,
            repairability_index: None,
            recycled_content_pct: None,
            compliance_status: ComplianceStatus::PassthroughNoValidation,
            violations: Vec::new(),
            warnings: Vec::new(),
            ruleset_version: None,
            assessed_at: None,
            receipt: None,
        }
    }
}

impl ComplianceResult {
    /// A passthrough determination: manufacturer values stored verbatim, no
    /// calculation performed.
    #[must_use]
    pub fn passthrough() -> Self {
        Self::default()
    }

    /// A determination carrying `status` with otherwise-empty fields.
    #[must_use]
    pub fn with_status(status: ComplianceStatus) -> Self {
        Self {
            compliance_status: status,
            ..Self::default()
        }
    }

    /// True if this determination carries any binding violation.
    #[must_use]
    pub fn has_violations(&self) -> bool {
        !self.violations.is_empty()
    }
}

/// Overall compliance determination for a passport.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum ComplianceStatus {
    /// Manufacturer-supplied values stored verbatim — no calculation performed.
    PassthroughNoValidation,
    /// Calculated and compliant with applicable EU regulation.
    Compliant,
    /// Calculated; one or more fields fall below regulatory thresholds.
    NonCompliant,
    /// The sector's DPP obligation is not yet in force (provisional), so no
    /// binding determination is legally applicable — only structural validation
    /// was performed. See [`gate_determination`].
    NotAssessed,
    /// Sector not yet implemented by this registry.
    NotImplemented,
}

impl ComplianceStatus {
    /// Every determination this build models, for exhaustive iteration.
    ///
    /// `ComplianceStatus` is `#[non_exhaustive]`, so a consumer outside this
    /// crate cannot enumerate it, and one publishing an API description has to.
    /// See [`crate::domain::seal::SealFormat::ALL`] for the same contract: a
    /// status added later is deliberately not covered until it is added here.
    pub const ALL: &'static [Self] = &[
        Self::PassthroughNoValidation,
        Self::Compliant,
        Self::NonCompliant,
        Self::NotAssessed,
        Self::NotImplemented,
    ];
}

/// Enforce regulatory status on a raw determination.
///
/// A product group no in-force act reaches may never surface a *binding*
/// `Compliant` / `NonCompliant` — there is no legal basis for the determination,
/// so it is downgraded to [`ComplianceStatus::NotAssessed`]. Groups an in-force
/// act does reach pass through unchanged, as do non-binding statuses.
///
/// Callers obtain `in_force` from
/// [`InstrumentCatalog::determinable_for`](crate::catalog::InstrumentCatalog::determinable_for),
/// which returns the (act, binding) pairs rather than a boolean. Pass the act
/// through to whatever records the result: a determination is always made under
/// a named instrument, and a caller that only learns "yes" cannot say which act
/// it is asserting against.
#[must_use]
pub fn gate_determination(in_force: bool, raw: ComplianceStatus) -> ComplianceStatus {
    if in_force {
        return raw;
    }
    match raw {
        ComplianceStatus::Compliant | ComplianceStatus::NonCompliant => {
            ComplianceStatus::NotAssessed
        }
        other => other,
    }
}

// ─── Error types ──────────────────────────────────────────────────────────

/// Error returned by a compliance strategy or registry.
#[derive(Debug, Clone)]
pub struct ComplianceError {
    pub kind: ComplianceErrorKind,
    pub message: String,
}

/// Classification of compliance calculation errors.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ComplianceErrorKind {
    /// No strategy registered for the requested sector.
    UnknownSector,
    /// Input sector data is structurally invalid for this strategy.
    InvalidInput,
    /// Internal error; should not propagate to the user.
    Internal,
}

impl fmt::Display for ComplianceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for ComplianceError {}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provisional_downgrades_binding_determinations() {
        assert_eq!(
            gate_determination(false, ComplianceStatus::Compliant),
            ComplianceStatus::NotAssessed
        );
        assert_eq!(
            gate_determination(false, ComplianceStatus::NonCompliant),
            ComplianceStatus::NotAssessed
        );
    }

    #[test]
    fn in_force_preserves_determinations() {
        assert_eq!(
            gate_determination(true, ComplianceStatus::Compliant),
            ComplianceStatus::Compliant
        );
        assert_eq!(
            gate_determination(true, ComplianceStatus::NonCompliant),
            ComplianceStatus::NonCompliant
        );
    }

    #[test]
    fn non_binding_statuses_pass_through_regardless() {
        for in_force in [true, false] {
            assert_eq!(
                gate_determination(in_force, ComplianceStatus::PassthroughNoValidation),
                ComplianceStatus::PassthroughNoValidation
            );
            assert_eq!(
                gate_determination(in_force, ComplianceStatus::NotAssessed),
                ComplianceStatus::NotAssessed
            );
        }
    }
}

#[cfg(test)]
mod compliance_status_all_tests {
    use super::ComplianceStatus;

    /// `ALL` must list every variant — see `PassportStatus`'s equivalent test.
    #[test]
    fn all_lists_every_variant() {
        for status in ComplianceStatus::ALL {
            match status {
                ComplianceStatus::PassthroughNoValidation
                | ComplianceStatus::Compliant
                | ComplianceStatus::NonCompliant
                | ComplianceStatus::NotAssessed
                | ComplianceStatus::NotImplemented => {}
            }
        }
        assert_eq!(
            ComplianceStatus::ALL.len(),
            5,
            "a variant was added to the match above but not to ALL"
        );
    }
}
