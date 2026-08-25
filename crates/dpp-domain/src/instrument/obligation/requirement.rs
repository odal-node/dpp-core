//! [`PassportObligation`] — whether an act requires a digital product passport,
//! and if not, why not.

use serde::{Deserialize, Serialize};

use super::date::ObligationDate;

/// Whether an instrument requires a digital product passport.
///
/// # The state the previous model could not express
///
/// A catalog entry used to carry `dppAppliesFrom: Option<String>`, so an act
/// either had a passport date or had not been given one yet. Two real and
/// distinct situations both collapsed into "no date":
///
/// - an act that creates obligations but **no passport at all** — ESPR Arts.
///   24–25 on unsold goods is a disclosure duty owed by an operator over a
///   financial year, with no product record anywhere in it; and
/// - an act whose passport is **displaced** by an equivalent digital system
///   under ESPR Art. 9(4)(b) — the working plan states that every product
///   covered by ecodesign measures gets a passport "except if there is an
///   alternative digital system providing equivalent information, for example
///   the EPREL database".
///
/// Because neither could be said, an adjacent act was recorded as `in_force`
/// with an inferred date, and a passport obligation that does not exist became
/// assertable. Making that state unrepresentable is the point of this type.
///
/// Serialised internally tagged on `obligation`:
/// `{"obligation":"required","from":{"date":"2027-02-18","basis":"sourced"}}`,
/// `{"obligation":"notRequired"}`,
/// `{"obligation":"displacedBy","system":"EPREL","basis":"ESPR Art. 9(4)(b)"}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "obligation", rename_all = "camelCase")]
#[non_exhaustive]
pub enum PassportObligation {
    /// The act requires a digital product passport. `from` is `None` where the
    /// act mandates one but no date is fixed — the position of every ESPR
    /// product group today, since no delegated act has been adopted.
    Required {
        /// When the obligation begins, if an act has fixed it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        from: Option<ObligationDate>,
    },
    /// The act imposes obligations but no passport. Not "no passport yet" —
    /// there is no passport article to wait for.
    NotRequired,
    /// The act's information duty is discharged through another system instead
    /// of a passport.
    DisplacedBy {
        /// The system that carries the information — e.g. `"EPREL"`.
        system: String,
        /// The legal basis for the displacement, e.g. `"ESPR Art. 9(4)(b)"`.
        basis: String,
    },
}

impl PassportObligation {
    /// Whether this act requires a passport at all.
    ///
    /// **Not** a determination gate. A determination is made under a named act
    /// and gated by that binding's
    /// [`RegulatoryStatus`](crate::catalog::RegulatoryStatus); this answers the
    /// separate question of whether the thing being determined is a *passport*
    /// obligation. Conflating the two is what let an adjacent act's real, live
    /// ecodesign duties be used to justify asserting a passport duty.
    #[must_use]
    pub fn is_required(&self) -> bool {
        matches!(self, Self::Required { .. })
    }

    /// The date the obligation begins, where an act has fixed one.
    ///
    /// `None` both for an undated requirement and for an act that requires no
    /// passport — callers wanting to tell those apart must match on the variant.
    #[must_use]
    pub fn applies_from(&self) -> Option<&ObligationDate> {
        match self {
            Self::Required { from } => from.as_ref(),
            _ => None,
        }
    }
}
