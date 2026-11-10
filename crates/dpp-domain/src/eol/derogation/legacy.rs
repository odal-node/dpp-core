//! [`DerogationRef`] — the free-text citation older end-of-life events carry.

use serde::{Deserialize, Serialize};

/// A free-text derogation citation, as end-of-life events were written before the
/// derogations were a closed list.
///
/// **Read-only history.** It stays so that an event written with it still reads and
/// writes back unchanged (see [`Derogation::Legacy`](super::Derogation::Legacy)).
/// Nothing in this crate checked `category` against the list of Commission
/// Delegated Regulation (EU) 2026/296 Art. 2, so it is never taken as one of its
/// points; cite a [`DiscardReason`](crate::unsold_goods::DiscardReason) instead.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DerogationRef {
    /// The derogation category, in whatever words the writer used.
    pub category: String,
    /// The act/article this derogation is grounded in (e.g. an OJ/CELEX ref).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub act_citation: Option<String>,
}
