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
