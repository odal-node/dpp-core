//! [`InstrumentStatus`] — how far through the legislative process an act is.

use serde::{Deserialize, Serialize};

/// How far through the legislative process an instrument is.
///
/// Distinct from [`RegulatoryStatus`](crate::catalog::RegulatoryStatus), and the
/// distinction is easy to lose: this says whether the **act exists**, that says
/// whether its obligations **bind a given product group**. An adopted act binds
/// nothing until its own dates arrive, and one act routinely binds one product
/// group while another waits — which is why the binding, not the instrument,
/// carries [`RegulatoryStatus`](crate::catalog::RegulatoryStatus).
///
/// The practical case: ESPR is `Adopted` and has been since 2024, while every
/// one of its product groups is still `Provisional` because no delegated act
/// exists. Both statements are true and neither implies the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum InstrumentStatus {
    /// Adopted and published in the Official Journal. Cite it by CELEX.
    Adopted,
    /// Formally proposed but not adopted. Its text can be read and cited as a
    /// *proposal*, never as law.
    Proposed,
    /// Announced in a working plan or otherwise expected, with no text to read.
    /// An instrument in this state has no CELEX, and nothing derived from it may
    /// be presented as sourced.
    Anticipated,
}

impl InstrumentStatus {
    /// Whether a text exists that can be cited by CELEX.
    ///
    /// Used by the catalog's own provenance test rather than by business logic:
    /// an instrument that claims to be adopted must name the act it was read
    /// from, or the claim has no basis.
    #[must_use]
    pub fn has_citable_text(&self) -> bool {
        matches!(self, Self::Adopted | Self::Proposed)
    }
}
