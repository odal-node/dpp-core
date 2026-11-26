//! Product-group data the tests share.

use super::super::*;

pub(super) fn minimal_battery_data() -> ProductGroupData {
    ProductGroupData::Battery(Box::new(BatteryData {
        due_diligence_url: Some("https://acme.example.com/due-diligence".into()),
        disassembly_instructions_url: Some("https://acme.example.com/disassembly".into()),
        ..crate::test_support::sample_battery_data()
    }))
}

pub(super) fn cotton_fibre(pct: f64) -> FibreEntry {
    FibreEntry {
        fibre: "cotton".into(),
        pct,
        country_of_origin: None,
    }
}

pub(super) fn polyester_fibre(pct: f64) -> FibreEntry {
    FibreEntry {
        fibre: "polyester".into(),
        pct,
        country_of_origin: None,
    }
}

pub(super) fn test_textile_data() -> TextileData {
    TextileData {
        fibre_composition: vec![cotton_fibre(60.0), polyester_fibre(40.0)],
        country_of_origin: "BD".into(),
        care_instructions: "Machine wash 40°C".into(),
        chemical_compliance_standard: "OEKO-TEX 100".into(),
        ..crate::test_support::sample_textile_data()
    }
}
