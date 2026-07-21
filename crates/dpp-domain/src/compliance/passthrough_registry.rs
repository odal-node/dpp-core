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
    use crate::domain::sector::{BatteryData, FibreEntry, SectorData, TextileData};
    use crate::ports::compliance::ComplianceStatus;

    fn battery_data() -> SectorData {
        SectorData::Battery(BatteryData {
            recycled_content_lithium_pct: Some(12.5),
            rated_capacity_kwh: Some(32.0),
            ..crate::test_support::sample_battery_data()
        })
    }

    fn textile_data() -> SectorData {
        SectorData::Textile(TextileData {
            fibre_composition: vec![FibreEntry {
                fibre: "cotton".into(),
                pct: 100.0,
                country_of_origin: None,
            }],
            country_of_origin: "BD".into(),
            care_instructions: "40°C wash".into(),
            chemical_compliance_standard: "OEKO-TEX 100".into(),
            recycled_content_pct: Some(30.0),
            carbon_footprint_kg_co2e: Some(8.5),
            repair_score: Some(7.5),
            ..crate::test_support::sample_textile_data()
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
