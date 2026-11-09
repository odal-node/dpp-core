//! [`ReceivingOperator`] — who the discarded products were delivered to.

use serde::{Deserialize, Serialize};

use crate::unsold_goods::LegalEntityIdentifier;

/// The operator that received the discarded products.
///
/// ESPR Art. 24(2) asks for the documentation "necessary to demonstrate the
/// delivery and reception of the discarded products", and Implementing Regulation
/// (EU) 2026/2 Annex III has an authority compare the disclosed treatment with
/// "treatment operations carried out by the waste treatment operator that received
/// the unsold products" — so a record has to say who that was.
///
/// The identifier is optional: no act requires a waste treatment operator to hold
/// one in a scheme this type could name. It reuses [`LegalEntityIdentifier`], the
/// same EUID-or-other-scheme shape the disclosing undertaking is identified by.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceivingOperator {
    /// The operator's name.
    pub name: String,
    /// How the operator is identified, where it has an identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identifier: Option<LegalEntityIdentifier>,
}
