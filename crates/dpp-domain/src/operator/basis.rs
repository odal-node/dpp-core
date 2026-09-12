//! [`ResponsibilityBasis`] — which law makes an operator answerable for a product.

use serde::{Deserialize, Serialize};

/// The legal basis under which an economic operator is the one responsible for
/// a product.
///
/// # Why this is recorded rather than derived
///
/// **ESPR Annex III, point (k)** does not name one law. Confirmed against the
/// verbatim OJ text, it asks for the operator "responsible for carrying out the
/// tasks set out in Article 4 of Regulation (EU) 2019/1020 **or** Article 15 of
/// Regulation (EU) 2023/988, **or similar tasks pursuant to other Union law
/// applicable to the product**" — a disjunction over three bases, and which one
/// applies is a fact about the product.
///
/// It is not derivable in general. **Art. 4(5) of Regulation (EU) 2019/1020**
/// limits that article to a closed list of instruments — verbatim from the
/// consolidated text (`02019R1020 — EN — 12.08.2026 — 003.001`): Regulations
/// (EU) No 305/2011, (EU) 2016/425, (EU) 2016/426, **(EU) 2023/1542** and
/// **(EU) 2024/1252**, and Directives 2000/14/EC, 2006/42/EC, 2009/48/EC,
/// 2009/125/EC, 2011/65/EU, 2013/29/EU, 2013/53/EU, 2014/29/EU, 2014/30/EU,
/// 2014/31/EU, 2014/32/EU, 2014/34/EU, 2014/35/EU, 2014/53/EU and 2014/68/EU.
///
/// The two emphasised Regulations put themselves on that list: batteries by
/// Reg. (EU) 2023/1542 (amendment marker `M1`, **28 July 2023**) and the
/// Critical Raw Materials Act by Reg. (EU) 2024/1252 (marker `M2`, 3 May 2024).
///
/// An earlier version of this note omitted both and concluded that "construction
/// (305/2011) and toys (2009/48/EC) fall inside it; the rest do not". It was
/// read off the **original** 2019/1020 rather than the consolidation, and it is
/// false: of the product groups this crate models, construction, toys **and
/// battery** fall inside, battery since 28 July 2023.
///
/// So the Art. 4 limb reaches more passports than that note implied. It is still
/// not derivable in general, which is the reason the basis is stated rather than
/// inferred — the correction narrows how wrong an inference would be, it does
/// not make inference safe.
///
/// Stating the basis is cheap and checkable. Inferring it from the product group
/// works for two groups and is a guess for every other.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum ResponsibilityBasis {
    /// Article 4 of Regulation (EU) 2019/1020 (market surveillance).
    ///
    /// Applies only to products subject to the instruments listed in Art. 4(5) —
    /// see the type-level note. The tasks are those in Art. 4(3): verifying the
    /// declaration of conformity and technical documentation, answering reasoned
    /// requests from market surveillance authorities, reporting a product
    /// believed to present a risk, and cooperating on corrective action.
    MarketSurveillanceArt4,
    /// Regulation (EU) 2023/988 (general product safety).
    ///
    /// **Annex III(k) cites "Article 15"**, and that is what the OJ prints —
    /// checked in both the original and the consolidated text. Article 15 of
    /// 2023/988 is *Cooperation of economic operators with market surveillance
    /// authorities*; the responsible-person provision is **Article 16**,
    /// *Responsible person for products placed on the Union market*.
    ///
    /// The discrepancy changes nothing here, because Art. 16(1) imports the
    /// other basis wholesale: "Article 4(2) and (3) of that Regulation shall
    /// apply to products covered by this Regulation." Both limbs therefore land
    /// on the same person performing the same tasks. This variant is named for
    /// the regulation rather than an article number so it does not have to
    /// choose between the citation as printed and the provision as intended.
    GeneralProductSafety,
    /// "Similar tasks pursuant to other Union law applicable to the product" —
    /// Annex III(k)'s own catch-all.
    ///
    /// Carries the citation because the whole point of the variant is that the
    /// law is not one of the two named, and a basis that cannot say which law it
    /// is says nothing at all.
    OtherUnionLaw {
        /// The instrument and provision, as it would be cited — for example
        /// `"Article 7 of Regulation (EU) 2017/745"`.
        citation: String,
    },
}

impl ResponsibilityBasis {
    /// Every basis this build models, for exhaustive iteration.
    ///
    /// Same contract as [`OperatorRole::ALL`](super::OperatorRole::ALL): the
    /// type is `#[non_exhaustive]`, so a consumer publishing an API description
    /// cannot enumerate it otherwise. [`Self::OtherUnionLaw`] appears with an
    /// empty citation — it is a shape, not a usable value.
    #[must_use]
    pub fn all() -> Vec<Self> {
        vec![
            Self::MarketSurveillanceArt4,
            Self::GeneralProductSafety,
            Self::OtherUnionLaw {
                citation: String::new(),
            },
        ]
    }
}
