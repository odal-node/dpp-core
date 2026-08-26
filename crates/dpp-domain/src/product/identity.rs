//! [`ProductIdentity`] — the compound key the import delta-matcher looks up by.

use serde::{Deserialize, Serialize};

use crate::passport::Passport;
use crate::product_group::ProductGroup;

/// Compound identity for matching an import row against an existing passport:
/// product group (dispatch key) + GTIN + optional batch.
///
/// Not a validated GS1 type — `gtin` is whatever string the product group's typed
/// data carries (only `Battery` validates it as a [`crate::identifier::Gtin`]; the
/// rest store it unchecked, and `UnsoldGoods`/`Other` carry none at all —
/// see [`super::product group::ProductGroupData::gtin`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductIdentity {
    pub product_group: ProductGroup,
    pub gtin: String,
    pub batch_id: Option<String>,
}

impl ProductIdentity {
    /// Derive the compound identity from a passport, or `None` if it has no
    /// product group data or its product group carries no GTIN field.
    pub fn from_passport(passport: &Passport) -> Option<Self> {
        let gtin = passport.product_group_data.as_ref()?.gtin()?.to_owned();
        Some(Self {
            product_group: passport.product_group.clone(),
            gtin,
            batch_id: passport.batch_id.clone(),
        })
    }
}
