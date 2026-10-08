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
/// # A third state, found later: content owed to someone else's passport
///
/// Regulation (EU) 2024/1252 Art. 28(6) creates no passport and displaces none,
/// and it is not "no passport" either: for a product that another act already
/// requires a passport for, the magnet information Art. 28(4) lists *"shall be
/// included in that product passport"*. [`NotRequired`](Self::NotRequired) is
/// literally true of it, and it hides what a reader of the catalog most needs to
/// know. The Act was first filed as a list of materials for that reason.
/// [`IncludedIn`](Self::IncludedIn) says it.
///
/// It is for an **operative** duty only. Regulation (EU) 2025/40 recital 70 says
/// a packaged product's passport "should also be used" for packaging
/// information, and a recital binds nobody, so that act stays `NotRequired`.
///
/// ✅ COMPLIANCE-PIN: EU 2024/1252, Art. 28(6) (OJ L, 2024/1252, 3.5.2024, as
/// corrected by Corrigendum 2024/90330): the sentence quoted above, and the only
/// place the Regulation mentions a passport. EU 2025/40, recital (70) (OJ L,
/// 2025/40, 22.1.2025): "that digital product passport should also be used for
/// providing the relevant information under this Regulation".
///
/// Serialised internally tagged on `obligation`:
/// `{"obligation":"required","from":{"date":"2027-02-18","basis":"sourced"}}`,
/// `{"obligation":"notRequired"}`,
/// `{"obligation":"displacedBy","system":"EPREL","basis":"ESPR Art. 9(4)(b)"}`,
/// `{"obligation":"includedIn","basis":"Regulation (EU) 2024/1252 Art. 28(6)"}`.
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
    /// The act imposes obligations but no passport, and requires nothing to be
    /// put into anyone else's. Not "no passport yet" — there is no passport
    /// article to wait for.
    NotRequired,
    /// The act creates no passport of its own, but requires information to be
    /// included in the passport **another Union act** requires for the same
    /// product. Where no other act requires one, the information is owed some
    /// other way, which the instrument's notes record.
    ///
    /// Not a determination gate and not a passport duty:
    /// [`is_required`](Self::is_required) is `false`, because no passport arises
    /// from this act. What it adds is the statement that a passport built under
    /// another act is incomplete without this act's content.
    IncludedIn {
        /// The provision that says so, e.g. `"Regulation (EU) 2024/1252 Art. 28(6)"`.
        basis: String,
    },
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
