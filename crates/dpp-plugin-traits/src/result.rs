//! Plugin call outcomes: [`PluginResult`] (typed compliance determination)
//! and [`AbiResult`] (the wire envelope carrying it or a [`PluginError`]).

use serde::{Deserialize, Serialize};

use crate::error::PluginError;

/// Typed compliance determination returned by a plugin.
///
/// Mirrors `dpp_domain::ports::compliance::ComplianceStatus` so the host maps
/// directly from one typed enum to the other rather than parsing a string.
/// Serialised as SCREAMING_SNAKE_CASE JSON strings for ABI stability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PluginComplianceStatus {
    Compliant,
    NonCompliant,
    NotAssessed,
    PassthroughNoValidation,
    NotImplemented,
}

/// Well-known metric key for CO₂e score (kg CO₂e per functional unit).
pub const METRIC_CO2E_SCORE: &str = "co2e_score";
/// Well-known metric key for the repairability index (0.0–10.0; non-regulatory
/// heuristic, not EN 45554 / EU 2023/1669).
pub const METRIC_REPAIRABILITY_INDEX: &str = "repairability_index";
/// Well-known metric key for recycled content percentage (0.0–100.0).
pub const METRIC_RECYCLED_CONTENT_PCT: &str = "recycled_content_pct";

/// A single determination finding emitted by a plugin's `calculate_metrics`.
///
/// Findings split into [`PluginResult::violations`] (binding — the host blocks
/// publish for an in-force sector) and [`PluginResult::warnings`]
/// (advisory/experimental — surfaced, never blocks). The vec encodes severity,
/// so there is no separate severity field. Maps 1:1 onto the host's
/// `ComplianceFinding`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginFinding {
    /// Stable machine-readable code, e.g. `"battery.recycled_content.cobalt_below_2031"`.
    pub code: String,
    /// JSON-pointer-style field locator (e.g. `"/recycledContentCobaltPct"`), or
    /// empty when the finding is not tied to a single field.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub field: String,
    /// Human-readable explanation.
    pub message: String,
}

impl PluginFinding {
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

/// Compliance result returned by the plugin.
///
/// `metrics` is a sector-extensible map of named numeric values. Use the
/// `METRIC_*` constants for the three well-known fields; plugins may add
/// sector-specific keys (`"water_use_litres"`, `"pef_score"`, …).
///
/// ## Aligning with `ComplianceResult` on the host
///
/// ```text
/// co2e_score           ← metrics["co2e_score"]
/// repairability_index  ← metrics["repairability_index"]
/// recycled_content_pct ← metrics["recycled_content_pct"]
/// compliance_status    ← PluginComplianceStatus → ComplianceStatus (typed map)
/// violations/warnings  ← PluginFinding → ComplianceFinding (host blocks on violations)
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginResult {
    /// Typed compliance determination.
    pub compliance_status: PluginComplianceStatus,
    /// Sector-extensible keyed metric map (all values finite f64).
    #[serde(default)]
    pub metrics: std::collections::HashMap<String, f64>,
    /// Non-numeric sector-specific data (free-form; stored verbatim in extra).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_json::Value>,
    /// Binding findings — the host blocks publish for an in-force sector. Empty
    /// for passthrough / not-assessed determinations. (ABI 1.1+)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub violations: Vec<PluginFinding>,
    /// Advisory / experimental findings — surfaced but never block publish (e.g.
    /// recycled-content thresholds that are not yet in force). (ABI 1.1+)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<PluginFinding>,
}

impl PluginResult {
    /// Construct a result carrying only a compliance status.
    pub fn new(status: PluginComplianceStatus) -> Self {
        Self {
            compliance_status: status,
            metrics: std::collections::HashMap::new(),
            extra: None,
            violations: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Insert a metric unconditionally (non-finite values are silently dropped).
    pub fn with_metric(mut self, key: &str, value: f64) -> Self {
        if value.is_finite() {
            self.metrics.insert(key.to_owned(), value);
        }
        self
    }

    /// Insert a metric only when `value` is `Some` (non-finite values dropped).
    pub fn maybe_metric(mut self, key: &str, value: Option<f64>) -> Self {
        if let Some(v) = value
            && v.is_finite()
        {
            self.metrics.insert(key.to_owned(), v);
        }
        self
    }

    /// Attach free-form non-numeric extra data.
    pub fn with_extra(mut self, extra: serde_json::Value) -> Self {
        self.extra = Some(extra);
        self
    }

    /// Append a binding violation (host blocks publish for an in-force sector).
    pub fn with_violation(mut self, finding: PluginFinding) -> Self {
        self.violations.push(finding);
        self
    }

    /// Append an advisory warning (surfaced, never blocks publish).
    pub fn with_warning(mut self, finding: PluginFinding) -> Self {
        self.warnings.push(finding);
        self
    }

    // ── Convenience accessors for the three well-known metrics ──────────────

    pub fn co2e_score(&self) -> Option<f64> {
        self.metrics.get(METRIC_CO2E_SCORE).copied()
    }

    pub fn repairability_index(&self) -> Option<f64> {
        self.metrics.get(METRIC_REPAIRABILITY_INDEX).copied()
    }

    pub fn recycled_content_pct(&self) -> Option<f64> {
        self.metrics.get(METRIC_RECYCLED_CONTENT_PCT).copied()
    }
}

/// JSON envelope wrapping the outcome of a fallible plugin ABI call.
///
/// The low-level Wasm exports (`validate`, `calculate_metrics`,
/// `generate_passport`) cannot return a Rust `Result` across the C ABI, so the
/// outcome is serialised as this externally-tagged enum. On success, `ok`
/// carries the method's return value (a [`PluginResult`] for
/// `calculate_metrics`, the normalised payload for `generate_passport`, or
/// `null` for `validate`). On failure, `error` carries a structured
/// [`PluginError`]. The host deserialises this to recover the typed result.
///
/// ```json
/// { "ok": { "co2eScore": 85.4, "complianceStatus": "NOT_ASSESSED", ... } }
/// { "error": { "ValidationErrors": [ { "field": "/gtin", "code": "missing", ... } ] } }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AbiResult {
    Ok(serde_json::Value),
    Error(PluginError),
}

impl AbiResult {
    /// Build a successful response by serialising `value`.
    pub fn ok<T: Serialize>(value: &T) -> Self {
        Self::Ok(serde_json::to_value(value).unwrap_or(serde_json::Value::Null))
    }

    /// Returns `true` if this is the success variant.
    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Ok(_))
    }
}
