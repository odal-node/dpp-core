//! [`ServiceProviderRef`] — who hosts the back-up copy.

use serde::{Deserialize, Serialize};

/// The digital product passport service provider hosting the back-up copy.
///
/// ✅ COMPLIANCE-PIN: ESPR (EU) 2024/1781 **Annex III point (l)** — *"the
/// reference of the digital product passport service provider hosting the
/// back-up copy of the digital product passport."*
///
/// # Why this exists beside the back-up URL rather than inside it
///
/// A URL is a location; this is a party, and the two are not recoverable from
/// each other. Two providers can serve from one domain, one provider can serve
/// from many, and a provider can be engaged with no link published at all.
///
/// **Art. 10(4) is what makes the pair inseparable:** *"The economic operator,
/// when placing the product on the market, shall make available a back-up copy
/// of the digital product passport **through a digital product passport service
/// provider**."* Unconditional, and it names the role. So a deployment that
/// publishes a back-up URL necessarily has a provider, and a request carrying
/// the one and not the other is incomplete by construction rather than by
/// choice.
///
/// # No scheme is mandated, deliberately
///
/// Annex III's second paragraph puts the data carrier, the point (b) product
/// identifier, the points (g)/(h)/(k) operator identifiers and the point (i)
/// facility identifiers under ISO/IEC 15459 conformity. **Point (l) is absent
/// from that list**, and Art. 2(32) defines the role as *"an independent
/// third-party authorised by the economic operator"* — a relationship, not a
/// registration. Requiring a scheme here would impose a conformity rule the
/// annex declines to.
///
/// 🚨 **A provider is verified, and being verified still does not give it an
/// identifier.** Implementing Regulation (EU) 2026/1778 recital 4 names the role
/// among value chain actors — *"each economic operator and value chain actor
/// (such as digital product passport service provider, repairer, refurbisher,
/// remanufacturer, recycler) should be identified through a verification
/// process"* — so **Art. 5** applies: a legal person obtains verified status by
/// proving identity and establishment with a qualified electronic seal or a
/// qualified electronic attestation of attributes. Art. 3(f) then has the
/// registry holding *"a list of verified digital product passport service
/// providers"*.
///
/// What that produces is a **status**, evidenced by a seal, not a namespace. Art.
/// 5 names the evidence and assigns no identifier, which is why point (l) still
/// mandates nothing and why the scheme here stays optional.
///
/// And verified status lapses: Art. 5(4) caps it at the expiry of the electronic
/// identification means or three years from verification, whichever comes first.
/// A reference is a record of who was named, not a standing assertion that they
/// are verified today.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceProviderRef {
    /// The provider's legal name.
    ///
    /// The one element the point can be read to compel: a reference that names
    /// nobody references nothing.
    pub name: String,
    /// The scheme [`value`](Self::value) is expressed in — `"vat"`, `"lei"`,
    /// `"eori"`, `"duns"`, `"did"` — where the provider holds an identifier.
    ///
    /// Carried explicitly for the reason
    /// [`RegistrationRequest::operator_identifier_scheme`](super::RegistrationRequest::operator_identifier_scheme)
    /// gives: a value alone does not say what it is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheme: Option<String>,
    /// The identifier value under [`scheme`](Self::scheme).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// ISO 3166-1 alpha-2 country of establishment, where known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
}

impl ServiceProviderRef {
    /// A reference carrying only the provider's name — the Annex III(l) floor.
    #[must_use]
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            scheme: None,
            value: None,
            country: None,
        }
    }
}
