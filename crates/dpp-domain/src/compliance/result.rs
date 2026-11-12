//! [`ComplianceResult`] — the whole determination for one product.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::finding::ComplianceFinding;
use super::status::ComplianceStatus;

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
    /// Binding findings — block publish when the product group is in force. Empty for
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
    /// The content hash of the product group key and `productGroupData` this
    /// determination was computed over. Set by `Passport::attach_compliance_result`,
    /// the one way to bind a determination. The first publish refuses a determination
    /// whose hash is missing or no longer matches the passport's content.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_hash: Option<String>,
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
            input_hash: None,
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

/// What a determination is computed over: exactly what
/// [`ComplianceRegistry::compute`](crate::ports::compliance::ComplianceRegistry::compute) reads.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ComplianceInput<'a, T: Serialize + ?Sized> {
    product_group_key: &'a str,
    product_group_data: Option<&'a T>,
}

/// The RFC 8785 content hash of a determination's input, as [`ComplianceResult::input_hash`]
/// records it. `data` is the product group's typed data, which this module does not name.
///
/// # Errors
/// Returns the canonicalisation error when the input cannot be hashed: a non-finite
/// number, or an integer beyond 2^53 - 1.
pub(crate) fn compliance_input_hash<T: Serialize + ?Sized>(
    product_group_key: &str,
    data: Option<&T>,
) -> Result<String, serde_json::Error> {
    dpp_rules::canonical::content_hash(&ComplianceInput {
        product_group_key,
        product_group_data: data,
    })
}
