//! Open-source passthrough compliance registry.
//!
//! Returns manufacturer-supplied data verbatim for **every** sector, computing
//! nothing. A *determination* (computed metrics, pass/fail) is the job of the
//! Wasm sector plugins (the canonical OSS determination path) or a proprietary
//!
//! This is the Apache-2.0 default. The [`ComplianceRegistry`] /
//! [`ComplianceStrategy`](crate::ports::compliance::ComplianceStrategy) traits
//! remain the extension seam a proprietary tier wires its own implementation
//! into.

use crate::{
    domain::sector::{Sector, SectorData},
    ports::compliance::{ComplianceError, ComplianceRegistry, ComplianceResult},
};

/// Open-source passthrough compliance registry.
///
/// Sector-agnostic: it makes no determination for any sector and computes no
/// metrics. Every sector yields
/// [`ComplianceStatus::PassthroughNoValidation`](crate::ports::compliance::ComplianceStatus::PassthroughNoValidation).
pub struct PassthroughRegistry;

impl PassthroughRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for PassthroughRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ComplianceRegistry for PassthroughRegistry {
    fn compute(
        &self,
        _sector: Sector,
        _data: &SectorData,
    ) -> Result<ComplianceResult, ComplianceError> {
        Ok(ComplianceResult::passthrough())
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::gtin::Gtin;
    use crate::domain::sector::{
        BatteryChemistry, BatteryData, FibreEntry, SectorData, TextileData,
    };
    use crate::ports::compliance::ComplianceStatus;

    fn battery_data() -> SectorData {
        SectorData::Battery(BatteryData {
            gtin: Gtin::parse("09506000134352").unwrap(),
            battery_chemistry: BatteryChemistry::Lfp,
            nominal_voltage_v: 3.2,
            nominal_capacity_ah: 100.0,
            expected_lifetime_cycles: 3000,
            co2e_per_unit_kg: 85.4,
            recycled_content_cobalt_pct: None,
            recycled_content_lithium_pct: Some(12.5),
            recycled_content_nickel_pct: None,
            state_of_health_pct: None,
            rated_capacity_kwh: Some(32.0),
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

    fn textile_data() -> SectorData {
        SectorData::Textile(TextileData {
            fibre_composition: vec![FibreEntry {
                fibre: "cotton".into(),
                pct: 100.0,
                country_of_origin: None,
            }],
            country_of_manufacturing: "BD".into(),
            care_instructions: "40°C wash".into(),
            chemical_compliance_standard: "OEKO-TEX 100".into(),
            recycled_content_pct: Some(30.0),
            carbon_footprint_kg_co2e: Some(8.5),
            water_use_litres: None,
            microplastic_shedding_mg_per_wash: None,
            repair_score: Some(7.5),
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
        })
    }

    #[test]
    fn passthrough_makes_no_determination_for_any_sector() {
        let registry = PassthroughRegistry::new();
        for (sector, data) in [
            (Sector::Battery, battery_data()),
            (Sector::Textile, textile_data()),
            // A sector with no per-sector handling used to return NotImplemented;
            // now it is handled uniformly.
            (Sector::Electronics, battery_data()),
        ] {
            let result = registry.compute(sector, &data).unwrap();
            assert_eq!(
                result.compliance_status,
                ComplianceStatus::PassthroughNoValidation
            );
            assert_eq!(result.co2e_score, None);
            assert_eq!(result.recycled_content_pct, None);
        }
    }
}
