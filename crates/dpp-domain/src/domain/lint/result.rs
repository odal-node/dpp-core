//! [`LintResult`] — the outcome of running the lint pack over a passport.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::finding::{LintFinding, lint_product_group_data};
use crate::domain::product_group::ProductGroupData;

/// The result of running the plausibility lint pack against a passport's
/// product group data. Never gates publish — see
/// [`crate::domain::passport::Passport::lint_result`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LintResult {
    /// The `dpp_rules::lint::LINT_PACK_VERSION` that produced `findings`.
    pub pack_version: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub findings: Vec<LintFinding>,
    pub assessed_at: DateTime<Utc>,
}

impl LintResult {
    /// Run the plausibility lint pack against `data`, stamping `assessed_at`
    /// as `Utc::now()`.
    #[must_use]
    pub fn compute(data: &ProductGroupData) -> Self {
        let now = Utc::now();
        Self {
            pack_version: dpp_rules::lint::LINT_PACK_VERSION.to_owned(),
            findings: lint_product_group_data(data, now),
            assessed_at: now,
        }
    }
}
