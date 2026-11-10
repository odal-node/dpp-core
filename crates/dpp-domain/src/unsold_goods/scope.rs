//! [`DisclosureScope`] — whether the disclosure covers one undertaking or a group.

use serde::{Deserialize, Serialize};

/// Whether this disclosure speaks for one undertaking or for a group.
///
/// **Annex I note (c) of Commission Implementing Regulation (EU) 2026/2:** "In
/// the case of a consolidated disclosure, the subsidiaries discarding unsold
/// consumer products **shall be listed** in addition to the parent undertaking.
/// In the case of other groups consisting of independent undertakings and a
/// central organisation supporting the group … with a common brand name,
/// consolidated disclosure may take place on a shared website, provided that the
/// member undertakings are listed."
///
/// The list is not optional decoration on the consolidated arm — without it a
/// reader cannot tell which undertakings a figure covers, and the same tonnage
/// could be disclosed by a parent and omitted by every subsidiary with nothing
/// visible. So it is a field of the variant, and a standalone disclosure has no
/// place to put one.
///
/// Note also what note (a) does to the name: for a subsidiary in a consolidated
/// disclosure, the entity name is **the parent's**, not the subsidiary's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
#[non_exhaustive]
pub enum DisclosureScope {
    /// One undertaking, disclosing for itself.
    Standalone,
    /// A parent undertaking disclosing for a group.
    Consolidated {
        /// The subsidiaries or member undertakings this disclosure covers.
        /// Required by note (c); an empty list is a malformed consolidated
        /// disclosure rather than a group of none.
        undertakings: Vec<String>,
    },
}
