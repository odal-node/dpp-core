//! [`DisclosingEntity`] — the header block of the Annex I disclosure.

use serde::{Deserialize, Serialize};

use super::identifier::LegalEntityIdentifier;
use super::scope::DisclosureScope;

/// Who is making the disclosure.
///
/// The first four rows of Annex I, Section 2. This is not passport data in the
/// ordinary sense — it identifies an **undertaking over a financial year**, not
/// a product placed on the market, which is the whole reason ESPR Arts. 24–25
/// impose no passport and this product-group slot exists only for
/// implementation convenience.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisclosingEntity {
    /// Annex I note (a): "either the name of the standalone undertaking or, for
    /// a subsidiary, **the name of the parent undertaking** of a group in the
    /// case of a consolidated disclosure."
    pub name: String,
    /// Note (b): the EUID, or another officially recognised scheme where no EUID
    /// is available.
    pub identifier: LegalEntityIdentifier,
    /// Note (c): standalone, or consolidated with its undertakings listed.
    pub scope: DisclosureScope,
}
