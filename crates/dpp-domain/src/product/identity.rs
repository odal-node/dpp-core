//! [`ProductIdentity`] — the compound key the import delta-matcher looks up by.

use serde::{Deserialize, Serialize};

use crate::passport::Passport;
use crate::product_group::ProductGroup;

/// Compound identity for matching an import row against an existing passport:
/// product group (dispatch key) + unique product identifier + optional batch.
///
/// # Why this is the identifier and not the GTIN
///
/// It was `gtin`, read from [`crate::product_group::ProductGroupData::gtin`].
/// Once an identifier could be issued under EN 18219 scheme 2 or 3, that
/// accessor answers `None` for a perfectly well-identified passport — so
/// `from_passport` returned `None`, the delta-matcher found no existing record,
/// and an import created a **duplicate** rather than updating the passport it
/// was looking at. A matching key must be built from the thing every passport
/// has, which is the identifier, not from the one scheme's value.
///
/// The string is
/// [`ProductIdentifier::as_str`](crate::identifier::ProductIdentifier::as_str),
/// so a scheme 1 identity is the 14-digit GTIN exactly as before and existing
/// keys are unchanged.
///
/// Not a validated type: this is a lookup key. `UnsoldGoods` and `Other` carry
/// no identifier at all and still produce no identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductIdentity {
    pub product_group: ProductGroup,
    /// The unique product identifier as a single string, whichever EN 18219
    /// clause 5 scheme issued it — the 14-digit GTIN for scheme 1.
    pub identifier: String,
    pub batch_id: Option<String>,
}

impl ProductIdentity {
    /// Derive the compound identity from a passport, or `None` if it has no
    /// product group data or its product group carries no identifier.
    pub fn from_passport(passport: &Passport) -> Option<Self> {
        let identifier = passport
            .product_group_data
            .as_ref()?
            .product_identifier()?
            .as_str()
            .to_owned();
        Some(Self {
            product_group: passport.product_group.clone(),
            identifier,
            batch_id: passport.batch_id.clone(),
        })
    }
}
