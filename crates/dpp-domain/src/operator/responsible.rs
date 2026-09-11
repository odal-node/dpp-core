//! [`ResponsibleOperator`] and its [`OperatorRole`].

use serde::{Deserialize, Serialize};

/// Identifies an economic operator responsible for a DPP.
///
/// Under ESPR, the "responsible economic operator" is whoever places or
/// makes the product available on the EU market. This can be the original
/// manufacturer, an importer, a distributor, or a remanufacturer.
///
/// # Contact details
///
/// [`postal_address`](Self::postal_address),
/// [`electronic_address`](Self::electronic_address) and
/// [`registered_trade_name`](Self::registered_trade_name) exist because four
/// separate provisions require the same four-part shape of whoever is
/// answerable for a product, and confirmed against the verbatim OJ text they
/// are: **ESPR Art. 27(6)** (manufacturer) and **Art. 29(3)** (importer) —
/// "name, registered trade name or registered trade mark, postal address at
/// which, and electronic means of communication through which, they can be
/// contacted"; **Art. 4(4) of Regulation (EU) 2019/1020** — "the name,
/// registered trade name or registered trade mark, and contact details,
/// including the postal address"; and **Art. 16(3) of Regulation (EU)
/// 2023/988** — the same, "including the postal and electronic address".
///
/// **ESPR Annex III, point (k)** is what puts them in the passport: "the name,
/// contact details and unique operator identifier of the economic operator
/// established in the Union responsible for carrying out the tasks set out in
/// Article 4 of Regulation (EU) 2019/1020 or Article 15 of Regulation (EU)
/// 2023/988, or similar tasks pursuant to other Union law applicable to the
/// product". Contact details are never a free string in any of the four.
///
/// ⚠️ **That quote's "Article 15" is the Regulation's own cross-reference, and
/// it points at the wrong article** — Art. 15 of 2023/988 is *Cooperation of
/// economic operators with market surveillance authorities*; the
/// responsible-person provision is Art. 16. The quote is left exactly as
/// enacted, and
/// [`ResponsibilityBasis::GeneralProductSafety`](crate::operator::ResponsibilityBasis::GeneralProductSafety)
/// carries the explanation, including why it changes nothing in substance.
///
/// All three are `Option` because this type is a persisted shape — it is stored
/// inside the transfer chain — and the envelope rule is additive-only. `None`
/// means not stated, never "has none".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponsibleOperator {
    /// The operator's DID (e.g. `did:web:acme.example.com`).
    pub did: String,
    /// Human-readable name of the economic operator.
    pub name: String,
    /// The operator's role in the supply chain.
    pub role: OperatorRole,
    /// EU-assigned economic operator identifier, if available.
    pub eu_operator_id: Option<String>,
    /// The scheme [`Self::eu_operator_id`] is expressed in — `"vat"`, `"lei"`,
    /// `"eori"`, `"duns"`. `None` when no EU identifier is held.
    ///
    /// Paired with the value because an identifier without its scheme cannot be
    /// stated truthfully to a registry: the value alone does not say what it is,
    /// and guessing produces a false claim rather than a missing one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eu_operator_id_scheme: Option<String>,
    /// ISO 3166-1 alpha-2 country code of the operator's establishment.
    pub country: String,
    /// Registered trade name or registered trade mark, where it differs from
    /// [`name`](Self::name).
    ///
    /// All four provisions name it as an alternative to the legal name — "name,
    /// registered trade name or registered trade mark" — because the two are
    /// frequently different and a reader holding the product sees the trading
    /// one. Kept as a separate field rather than overloading `name`, which is
    /// the legal name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registered_trade_name: Option<String>,
    /// Postal address at which the operator can be contacted.
    ///
    /// ESPR Art. 27(6) adds a constraint the other three do not: "The address
    /// shall indicate a single point where the manufacturer can be contacted."
    /// That is a property of the content, not of the shape, so nothing here
    /// enforces it — a free string cannot express "single point", and a
    /// structured address still could not say whether it is the only one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub postal_address: Option<String>,
    /// Electronic means of communication through which the operator can be
    /// contacted — an email address, or a contact URL.
    ///
    /// Distinct from [`did`](Self::did), and deliberately: a DID document
    /// resolves keys. Nothing in it is required to be a mailbox or a form, so it
    /// is not a channel through which a person can be contacted, and reading it
    /// as one would satisfy the obligation on paper and not in fact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub electronic_address: Option<String>,
}

/// The role of an economic operator in the DPP supply chain.
///
/// Determines what DPP fields the operator may introduce or update,
/// as specified by the applicable delegated act.
///
/// # Wider than Art. 4(2), on purpose
///
/// **Art. 4(2) of Regulation (EU) 2019/1020** defines a closed set of four who
/// may be *the* responsible operator: a manufacturer established in the Union;
/// an importer, where the manufacturer is not; an authorised representative
/// with a written mandate; and a fulfilment service provider established in the
/// Union, where no other is. This enum holds those four and five more, because
/// it also describes who a passport can be *transferred to* — a remanufacturer
/// or a preparer for reuse takes over responsibility without being an Art. 4(2)
/// designation, and a distributor appears in the chain while never being able
/// to be the responsible operator at all.
///
/// [`ResponsibilityBasis`](super::ResponsibilityBasis) is what narrows the set
/// where the narrowing matters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum OperatorRole {
    /// Original equipment manufacturer.
    Manufacturer,
    /// Imports the product into the EU market.
    Importer,
    /// Makes the product available on the market without altering it.
    Distributor,
    /// An EU-established entity authorised to act on behalf of a
    /// non-EU manufacturer.
    AuthorisedRepresentative,
    /// An EU-established provider of warehousing, packaging, addressing or
    /// dispatching for products it does not own.
    ///
    /// Art. 4(2)(d) of Regulation (EU) 2019/1020 makes this operator the
    /// responsible one **only** "where no other economic operator as mentioned
    /// in points (a), (b) and (c) is established in the Union" — it is the
    /// fallback that stops a product having no answerable party at all. Missing
    /// from this enum until 2026-09-07, which left that case unexpressible.
    FulfilmentServiceProvider,
    /// Performs remanufacturing — restores the product to original
    /// or improved specifications.
    Remanufacturer,
    /// Adapts the product for a different purpose than originally intended.
    Repurposer,
    /// Prepares a used product for resale (testing, cleaning, repair).
    PreparerForReuse,
    /// Professional repairer with authorised DPP update rights.
    Repairer,
    /// Processes end-of-life products for material recovery.
    Recycler,
}

impl OperatorRole {
    /// Every role this build models, for exhaustive iteration.
    ///
    /// `OperatorRole` is `#[non_exhaustive]`, so a consumer outside this crate
    /// cannot enumerate it, and one publishing an API description has to. See
    /// [`crate::seal::SealFormat::ALL`] for the same contract: a role
    /// added later is deliberately not covered until it is added here.
    pub const ALL: &'static [Self] = &[
        Self::Manufacturer,
        Self::Importer,
        Self::Distributor,
        Self::AuthorisedRepresentative,
        Self::FulfilmentServiceProvider,
        Self::Remanufacturer,
        Self::Repurposer,
        Self::PreparerForReuse,
        Self::Repairer,
        Self::Recycler,
    ];

    /// Whether this role can be the responsible operator under Art. 4(2) of
    /// Regulation (EU) 2019/1020, which names four and only four.
    ///
    /// Advisory rather than enforced: (a) and (b) carry conditions this type
    /// cannot see — "established in the Union", "where the manufacturer is not
    /// established in the Union" — and (d) is conditional on the absence of the
    /// other three. So a `true` here means "not excluded by role", never "is the
    /// Art. 4(2) operator".
    #[must_use]
    pub const fn can_be_art_4_operator(&self) -> bool {
        matches!(
            self,
            Self::Manufacturer
                | Self::Importer
                | Self::AuthorisedRepresentative
                | Self::FulfilmentServiceProvider
        )
    }
}
