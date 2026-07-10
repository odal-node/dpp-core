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
    use crate::domain::gtin::Gtin;
    use crate::domain::passport::ManufacturerInfo;
    use crate::domain::sector::{BatteryChemistry, BatteryData, SectorData, TextileData};
    use crate::domain::status::PassportStatus;

    fn base_passport(sector: Sector, sector_data: Option<SectorData>) -> Passport {
        Passport {
            id: crate::domain::passport::PassportId::new(),
            batch_id: Some("BATCH-1".into()),
            product_name: "Test".into(),
            sector,
            product_category: None,
            manufacturer: ManufacturerInfo {
                name: "Acme".into(),
                address: "1 Street".into(),
                did_web_url: None,
            },
            materials: vec![],
            co2e_per_unit: None,
            repairability_score: None,
            compliance_result: None,
            lint_result: None,
            sector_data,
            status: PassportStatus::Draft,
            qr_code_url: None,
            jws_signature: None,
            public_jws_signature: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            published_at: None,
            schema_version: "1.0.0".into(),
            retention_locked: false,
            version: 1,
            supersedes_id: None,
            retention_until: None,
            product_id: None,
            operator_identifier: None,
            facility: None,
            seal: None,
        }
    }

    fn battery_data() -> SectorData {
        SectorData::Battery(BatteryData {
            gtin: Gtin::parse("09506000134352").unwrap(),
            battery_chemistry: BatteryChemistry::Lfp,
            nominal_voltage_v: 3.2,
            nominal_capacity_ah: 100.0,
            expected_lifetime_cycles: 3000,
            co2e_per_unit_kg: 85.4,
            recycled_content_cobalt_pct: None,
            recycled_content_lithium_pct: None,
            recycled_content_nickel_pct: None,
            state_of_health_pct: None,
            rated_capacity_kwh: None,
            carbon_footprint_class: None,
            due_diligence_url: None,
            cathode_material: None,
            anode_material: None,
            electrolyte_material: None,
            critical_raw_materials: None,
            disassembly_instructions_url: None,
            soh_methodology: None,
            operating_temp_min_c: None,
            operating_temp_max_c: None,
            rated_energy_wh: None,
            recycled_content_lead_pct: None,
            battery_weight_kg: None,
            battery_type: None,
            round_trip_efficiency_pct: None,
            internal_resistance_mohm: None,
            manufacturing_date: None,
            manufacturing_place: None,
            battery_model_id: None,
            battery_passport_number: None,
        })
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
            gtin: "09506000134352".into(),
            fibre_composition: vec![],
            country_of_manufacturing: "BD".into(),
            care_instructions: "wash".into(),
            chemical_compliance_standard: "OEKO-TEX 100".into(),
            recycled_content_pct: None,
            carbon_footprint_kg_co2e: None,
            water_use_litres: None,
            microplastic_shedding_mg_per_wash: None,
            repair_score: None,
            durability_score: None,
            expected_wash_cycles: None,
            country_of_raw_material_origin: None,
            svhc_substances: None,
            allergens: None,
            substances_of_concern: None,
            recyclability_class: None,
            end_of_life_instructions: None,
            reuse_condition: None,
            prior_use_cycles: None,
            disassembly_instructions: None,
            spare_parts_available: None,
            product_weight_grams: None,
            repair_history_url: None,
            repair_count: None,
            pef_score: None,
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
