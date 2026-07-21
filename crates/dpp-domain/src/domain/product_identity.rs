//! [`ProductIdentity`] — the compound key the import delta-matcher looks up by.

use serde::{Deserialize, Serialize};

use super::passport::Passport;
use super::sector::Sector;

/// Compound identity for matching an import row against an existing passport:
/// sector (dispatch key) + GTIN + optional batch.
///
/// Not a validated GS1 type — `gtin` is whatever string the sector's typed
/// data carries (only `Battery` validates it as a [`super::gtin::Gtin`]; the
/// rest store it unchecked, and `UnsoldGoods`/`Other` carry none at all —
/// see [`super::sector::SectorData::gtin`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductIdentity {
    pub sector: Sector,
    pub gtin: String,
    pub batch_id: Option<String>,
}

impl ProductIdentity {
    /// Derive the compound identity from a passport, or `None` if it has no
    /// sector data or its sector carries no GTIN field.
    pub fn from_passport(passport: &Passport) -> Option<Self> {
        let gtin = passport.sector_data.as_ref()?.gtin()?.to_owned();
        Some(Self {
            sector: passport.sector.clone(),
            gtin,
            batch_id: passport.batch_id.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::passport::ManufacturerInfo;
    use crate::domain::sector::{SectorData, TextileData};

    fn base_passport(sector: Sector, sector_data: Option<SectorData>) -> Passport {
        Passport {
            batch_id: Some("BATCH-1".into()),
            product_name: "Test".into(),
            sector,
            manufacturer: ManufacturerInfo {
                name: "Acme".into(),
                address: "1 Street".into(),
                did_web_url: None,
            },
            sector_data,
            ..crate::test_support::sample_passport()
        }
    }

    fn battery_data() -> SectorData {
        SectorData::Battery(crate::test_support::sample_battery_data())
    }

    #[test]
    fn battery_passport_yields_identity() {
        let p = base_passport(Sector::Battery, Some(battery_data()));
        let id = ProductIdentity::from_passport(&p).expect("battery has a gtin");
        assert_eq!(id.sector, Sector::Battery);
        assert_eq!(id.gtin, "09506000134352");
        assert_eq!(id.batch_id.as_deref(), Some("BATCH-1"));
    }

    #[test]
    fn textile_passport_yields_identity() {
        let textile_data = SectorData::Textile(TextileData {
            country_of_manufacturing: "BD".into(),
            care_instructions: "wash".into(),
            chemical_compliance_standard: "OEKO-TEX 100".into(),
            ..crate::test_support::sample_textile_data()
        });
        let p = base_passport(Sector::Textile, Some(textile_data));
        let id = ProductIdentity::from_passport(&p).expect("textile has a gtin");
        assert_eq!(id.sector, Sector::Textile);
        assert_eq!(id.gtin, "09506000134352");
    }

    #[test]
    fn no_sector_data_yields_no_identity() {
        let p = base_passport(Sector::Battery, None);
        assert!(ProductIdentity::from_passport(&p).is_none());
    }
}
