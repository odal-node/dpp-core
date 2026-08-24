//! [`ResponsibleOperator`] and its [`OperatorRole`].

use serde::{Deserialize, Serialize};

/// Identifies an economic operator responsible for a DPP.
///
/// Under ESPR, the "responsible economic operator" is whoever places or
/// makes the product available on the EU market. This can be the original
/// manufacturer, an importer, a distributor, or a remanufacturer.
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
}

/// The role of an economic operator in the DPP supply chain.
///
/// Determines what DPP fields the operator may introduce or update,
/// as specified by the applicable delegated act.
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
    /// [`crate::domain::seal::SealFormat::ALL`] for the same contract: a role
    /// added later is deliberately not covered until it is added here.
    pub const ALL: &'static [Self] = &[
        Self::Manufacturer,
        Self::Importer,
        Self::Distributor,
        Self::AuthorisedRepresentative,
        Self::Remanufacturer,
        Self::Repurposer,
        Self::PreparerForReuse,
        Self::Repairer,
        Self::Recycler,
    ];
}

#[cfg(test)]
mod operator_role_all_tests {
    use super::OperatorRole;

    /// `ALL` must list every variant.
    ///
    /// The match below is exhaustive and has no catch-all, so adding a variant
    /// stops this file compiling — and the length assertion then fails until
    /// `ALL` is updated too. Two stages, because a const list that can silently
    /// fall behind the enum is worse than no list: every consumer that trusts it
    /// to be complete inherits the gap.
    #[test]
    fn all_lists_every_variant() {
        for role in OperatorRole::ALL {
            match role {
                OperatorRole::Manufacturer
                | OperatorRole::Importer
                | OperatorRole::Distributor
                | OperatorRole::AuthorisedRepresentative
                | OperatorRole::Remanufacturer
                | OperatorRole::Repurposer
                | OperatorRole::PreparerForReuse
                | OperatorRole::Repairer
                | OperatorRole::Recycler => {}
            }
        }
        assert_eq!(
            OperatorRole::ALL.len(),
            9,
            "a variant was added to the match above but not to ALL"
        );
    }
}
