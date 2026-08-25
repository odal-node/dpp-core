//! [`ComplianceFinding`] — one rule outcome attached to a determination.

use serde::{Deserialize, Serialize};

/// A single compliance finding (one rule outcome) attached to a determination.
///
/// Findings are split into [`ComplianceResult::violations`] (binding — block
/// publish for an in-force product group) and [`ComplianceResult::warnings`]
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
