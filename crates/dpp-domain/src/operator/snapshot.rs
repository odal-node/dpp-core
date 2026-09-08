//! [`ResponsibleOperatorSnapshot`] — Annex III(k)'s three elements, in the passport.

use serde::{Deserialize, Serialize};

use super::{ResponsibilityBasis, ResponsibleOperator};

/// The economic operator answerable for a product, copied into the passport by
/// value.
///
/// # What it satisfies
///
/// **ESPR Annex III, point (k)** asks the passport to carry "the name, contact
/// details and unique operator identifier of the economic operator established
/// in the Union responsible for carrying out" the relevant tasks. All three live
/// on [`ResponsibleOperator`]; [`basis`](Self::basis) records which law makes
/// this operator the responsible one, which point (k) leaves open as a
/// disjunction rather than fixing.
///
/// # Why a snapshot, and not a reference to the transfer chain
///
/// The chain is the authoritative history of who has been responsible; this is
/// the passport's own statement of who is responsible for the version a reader
/// is holding. Copying by value is the same choice
/// [`FacilitySnapshot`](crate::facility::FacilitySnapshot) makes and for the
/// same reason: a signed passport stays a complete record, independent of a
/// registry that can change underneath it.
///
/// # Why it is not frozen the way `operator_identifier` is
///
/// **ESPR Art. 9(1)**, final sentence: "The data in the digital product passport
/// shall be accurate, complete and up to date." So point (k) wants whoever is
/// responsible *now*, not at first publication —
/// [`Passport::operator_identifier`](crate::passport::Passport::operator_identifier)
/// is the publisher frozen at publish and is a different fact.
///
/// A published passport's content is immutable, so "up to date" cannot mean
/// rewriting this in place. It means the transfer that moves responsibility is
/// followed by a corrected successor carrying the new operator — the record of
/// the handover stays in the chain, and the reader of any single version sees
/// the operator that version was issued under.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponsibleOperatorSnapshot {
    /// The operator: name, role, identifier and contact details.
    pub operator: ResponsibleOperator,
    /// Which law makes this operator the responsible one.
    ///
    /// Not on [`ResponsibleOperator`] itself, deliberately. The same company can
    /// be an Art. 4 responsible operator for one product and a general
    /// product-safety responsible person for another, so the basis is a property
    /// of this pairing rather than of the operator — and a transfer record,
    /// which holds two operators, would otherwise have to carry two copies of a
    /// fact that belongs to neither.
    pub basis: ResponsibilityBasis,
}

impl ResponsibleOperatorSnapshot {
    /// Whether the operator's role is one Art. 4(2) of Regulation (EU)
    /// 2019/1020 admits, where that is the basis claimed.
    ///
    /// `true` for any other basis: only the Art. 4 limb has a closed role set,
    /// so a general product-safety or other-Union-law operator is not
    /// constrained by it here. Advisory — see
    /// [`OperatorRole::can_be_art_4_operator`](super::OperatorRole::can_be_art_4_operator)
    /// for the conditions the role alone cannot check.
    #[must_use]
    pub fn role_fits_basis(&self) -> bool {
        match self.basis {
            ResponsibilityBasis::MarketSurveillanceArt4 => {
                self.operator.role.can_be_art_4_operator()
            }
            _ => true,
        }
    }
}
