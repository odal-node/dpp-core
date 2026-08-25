//! [`RegistryIdentifiers`] — the persistent identifiers a registration carries.

use serde::{Deserialize, Serialize};

/// The persistent identifiers a registration carries. Specified by ESPR
/// **Annex III** (product (b), operator (g)/(h), facility (i)); Art. 13 is the
/// registry that stores them, not their definition.
///
/// Every product registered in the EU Central Registry receives four
/// identifiers that persist throughout its lifecycle, even across
/// ownership transfers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryIdentifiers {
    /// Unique product identifier within the EU registry.
    pub product_id: String,
    /// Identifier of the economic operator who placed the product on the market.
    pub operator_id: String,
    /// Identifier of the facility where the product was manufactured or imported.
    pub facility_id: String,
    /// The registry's own record identifier.
    pub registry_id: String,
}
