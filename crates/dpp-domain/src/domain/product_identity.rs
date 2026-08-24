//! [`ProductIdentity`] — the compound key the import delta-matcher looks up by.

use serde::{Deserialize, Serialize};

use super::passport::Passport;
use super::product_group::ProductGroup;

/// Compound identity for matching an import row against an existing passport:
/// product group (dispatch key) + GTIN + optional batch.
///
/// Not a validated GS1 type — `gtin` is whatever string the product group's typed
/// data carries (only `Battery` validates it as a [`super::gtin::Gtin`]; the
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::passport::ManufacturerInfo;
    use crate::domain::product_group::{ProductGroupData, TextileData};

    fn base_passport(
        product_group: ProductGroup,
        product_group_data: Option<ProductGroupData>,
    ) -> Passport {
        Passport {
            batch_id: Some("BATCH-1".into()),
            product_name: "Test".into(),
            product_group,
            manufacturer: ManufacturerInfo {
                name: "Acme".into(),
                address: "1 Street".into(),
                did_web_url: None,
            },
            product_group_data,
            ..crate::test_support::sample_passport()
        }
    }

    fn battery_data() -> ProductGroupData {
        ProductGroupData::Battery(Box::new(crate::test_support::sample_battery_data()))
    }

    #[test]
    fn battery_passport_yields_identity() {
        let p = base_passport(ProductGroup::Battery, Some(battery_data()));
        let id = ProductIdentity::from_passport(&p).expect("battery has a gtin");
        assert_eq!(id.product_group, ProductGroup::Battery);
        assert_eq!(id.gtin, "09506000134352");
        assert_eq!(id.batch_id.as_deref(), Some("BATCH-1"));
    }

    #[test]
    fn textile_passport_yields_identity() {
        let textile_data = ProductGroupData::Textile(Box::new(TextileData {
            country_of_origin: "BD".into(),
            care_instructions: "wash".into(),
            chemical_compliance_standard: "OEKO-TEX 100".into(),
            ..crate::test_support::sample_textile_data()
        }));
        let p = base_passport(ProductGroup::Textile, Some(textile_data));
        let id = ProductIdentity::from_passport(&p).expect("textile has a gtin");
        assert_eq!(id.product_group, ProductGroup::Textile);
        assert_eq!(id.gtin, "09506000134352");
    }

    #[test]
    fn no_product_group_data_yields_no_identity() {
        let p = base_passport(ProductGroup::Battery, None);
        assert!(ProductIdentity::from_passport(&p).is_none());
    }
}
