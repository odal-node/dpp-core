//! [`DerogationRef`] — the lawful basis a destruction claim must cite.

use serde::{Deserialize, Serialize};

/// A recognised derogation from the ESPR Art. 25 destruction ban. The exact
/// category list is fixed by the Feb-2026 delegated act; the category string is
/// validated against that list at the engine boundary, not here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DerogationRef {
    /// The derogation category as named by the delegated act.
    pub category: String,
    /// The act/article this derogation is grounded in (e.g. an OJ/CELEX ref).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub act_citation: Option<String>,
}
