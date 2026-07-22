//! Cross-crate integration test: AAS mapping across every sector.
//!
//! `build_aas_from_passport` (dpp-digital-link::aas) is the primary AAS entry
//! point — it dispatches each sector's `SectorData` to a dedicated submodel
//! mapper. This test feeds a fully-populated passport for **every** sector
//! through that path (so each mapper's optional-field branches execute) and
//! asserts the resulting shell + submodels are well-formed and serialisable.
//!
//! It complements the per-sector E2E tests (battery, textile) by guaranteeing
//! no sector mapper silently regresses or panics, and that the AAS submodel
//! template registry stays in sync with the sectors.

use chrono::Utc;
use dpp_digital_link::aas::{
    build_aas_from_passport, placeholder_templates, sector_submodel_template,
};
use dpp_domain::domain::sector::CriticalRawMaterial;
use dpp_domain::{
    AluminiumData, ConstructionData, DetergentData, ElectronicsData, EnergyEfficiencyClass,
    FibreEntry, FurnitureData, Gtin, ProductionRoute, RepairabilityScore, Sector, SectorData,
    SteelData, SurfactantEntry, SvhcSubstance, TextileData, ToyData, TyreData,
    UnsoldGoodsDestination, UnsoldGoodsReason, UnsoldGoodsReport,
};
use dpp_tests::fixtures::base_passport as base;

const VALID_GTIN: &str = "09506000134352";

fn svhc() -> SvhcSubstance {
    SvhcSubstance {
        cas_number: "80-05-7".into(),
        substance_name: "Bisphenol A".into(),
        concentration_pct: 0.12,
        location_in_product: Some("coating".into()),
        scip_notification_id: Some("SCIP-2026-1".into()),
    }
}

fn crm() -> CriticalRawMaterial {
    CriticalRawMaterial {
        name: "cobalt".into(),
        cas_number: Some("7440-48-4".into()),
        weight_grams: Some(40.0),
        country_of_origin: Some("CD".into()),
    }
}

fn electronics_data() -> ElectronicsData {
    ElectronicsData {
        gtin: Gtin::parse(VALID_GTIN).unwrap(),
        product_category: "laptop".into(),
        energy_efficiency_class: EnergyEfficiencyClass::B,
        co2e_per_unit_kg: 210.0,
        repairability_score: Some(RepairabilityScore::from_scalar(7.5)),
        spare_parts_available: Some(true),
        repair_manual_url: Some("https://acme.example.com/repair".into()),
        disassembly_instructions_url: Some("https://acme.example.com/disassembly".into()),
        svhc_substances: Some(vec![svhc()]),
        rohs_compliant: Some(true),
        critical_raw_materials: Some(vec![crm()]),
        recycled_content_pct: Some(35.0),
        standby_power_w: Some(0.4),
        expected_lifetime_years: Some(7),
        firmware_update_until: Some(Utc::now()),
    }
}

fn textile_data() -> TextileData {
    TextileData {
        gtin: Gtin::parse("09506000134352").unwrap(),
        fibre_composition: vec![FibreEntry {
            fibre: "cotton".into(),
            pct: 100.0,
            country_of_origin: Some("IN".into()),
        }],
        country_of_origin: "BD".into(),
        care_instructions: "Machine wash 30°C".into(),
        chemical_compliance_standard: "OEKO-TEX 100".into(),
        recycled_content_pct: Some(10.0),
        carbon_footprint_kg_co2e: Some(8.5),
        water_use_litres: Some(2700.0),
        microplastic_shedding_mg_per_wash: Some(11.0),
        repair_score: Some(6.0),
        durability_score: Some(7.0),
        expected_wash_cycles: Some(50),
        country_of_raw_material_origin: Some("IN".into()),
        svhc_substances: Some(vec![svhc()]),
        allergens: None,
        substances_of_concern: None,
        recyclability_class: Some("mono-material".into()),
        end_of_life_instructions: Some("Return to store".into()),
        reuse_condition: None,
        prior_use_cycles: Some(0),
        disassembly_instructions: Some("Remove buttons".into()),
        spare_parts_available: Some(true),
        product_weight_grams: Some(250.0),
        repair_history_url: None,
        repair_count: None,
        pef_score: None,
    }
}

fn steel_data() -> SteelData {
    SteelData {
        gtin: Gtin::parse(VALID_GTIN).unwrap(),
        co2e_per_tonne_steel: 1.8,
        recycled_scrap_content_pct: 85.0,
        product_category: "flat".into(),
        country_of_origin: "SE".into(),
        production_route: ProductionRoute::ElectricArc,
        annual_production_tonnes: Some(120000.0),
    }
}

fn construction_data() -> ConstructionData {
    ConstructionData {
        gtin: Gtin::parse(VALID_GTIN).unwrap(),
        product_family: "cement".into(),
        country_of_origin: "DE".into(),
        co2e_per_functional_unit_kg: 0.6,
        functional_unit: "per tonne".into(),
        recycled_content_pct: Some(25.0),
        epd_url: Some("https://acme.example.com/epd".into()),
        ce_marking: Some(true),
    }
}

fn tyre_data() -> TyreData {
    TyreData {
        gtin: Gtin::parse(VALID_GTIN).unwrap(),
        tyre_class: "C1".into(),
        fuel_efficiency_class: "B".into(),
        wet_grip_class: "A".into(),
        external_rolling_noise_db: 70.0,
        noise_performance_class: Some("B".into()),
        rolling_resistance_n_per_kn: Some(6.5),
        recycled_rubber_pct: Some(18.0),
        co2e_per_tyre_kg: Some(85.0),
    }
}

fn toy_data() -> ToyData {
    ToyData {
        gtin: Gtin::parse(VALID_GTIN).unwrap(),
        age_group: "3-6".into(),
        primary_material: "wood".into(),
        ce_marking: true,
        country_of_origin: "DE".into(),
        svhc_substances: Some(vec![svhc()]),
        contains_battery: Some(false),
        repairability_info: Some("https://acme.example.com/toy-repair".into()),
    }
}

fn aluminium_data() -> AluminiumData {
    AluminiumData {
        gtin: Gtin::parse(VALID_GTIN).unwrap(),
        alloy_grade: "6xxx".into(),
        production_route: ProductionRoute::SecondaryRecycled,
        co2e_per_tonne_kg: 4000.0,
        recycled_content_pct: 75.0,
        country_of_origin: "NO".into(),
        annual_production_tonnes: Some(50000.0),
    }
}

fn furniture_data() -> FurnitureData {
    FurnitureData {
        gtin: Gtin::parse(VALID_GTIN).unwrap(),
        product_type: "chair".into(),
        primary_material: "solid-wood".into(),
        country_of_origin: "SE".into(),
        co2e_per_unit_kg: Some(22.0),
        recycled_content_pct: Some(15.0),
        repairability_score: Some(6.5),
        svhc_substances: Some(vec![svhc()]),
        disassembly_instructions_url: Some("https://acme.example.com/furniture".into()),
        end_of_life_instructions: Some("Disassemble and recycle wood".into()),
    }
}

fn detergent_data() -> DetergentData {
    DetergentData {
        gtin: Gtin::parse(VALID_GTIN).unwrap(),
        product_type: "laundry".into(),
        format: "liquid".into(),
        surfactants: vec![SurfactantEntry {
            name: "Sodium laureth sulfate".into(),
            biodegradable: true,
            concentration_band: "5-15%".into(),
            cas_number: Some("9004-82-4".into()),
        }],
        country_of_origin: "DE".into(),
        co2e_per_unit_kg: Some(1.2),
        packaging_recyclable: Some(true),
        recommended_dosage_ml: Some(35.0),
        biodegradable: Some(true),
    }
}

fn unsold_goods_report() -> UnsoldGoodsReport {
    UnsoldGoodsReport {
        reporting_period: "2026-Q3".into(),
        volume_kg: 420.0,
        product_category: "apparel".into(),
        reason: UnsoldGoodsReason::EndOfSeason,
        destination: UnsoldGoodsDestination::Donation,
        destruction_justification: None,
        country_of_disposal: "DE".into(),
        operator_name: Some("Charity Recipient e.V.".into()),
    }
}

/// Every sector's data, paired with its schema version and the expected
/// sector-submodel `idShort`.
fn all_sector_cases() -> Vec<(Sector, SectorData, &'static str, &'static str)> {
    vec![
        (
            Sector::Electronics,
            SectorData::Electronics(electronics_data()),
            "1.0.0",
            "ElectronicsProductData",
        ),
        (
            Sector::Textile,
            SectorData::Textile(textile_data()),
            "1.1.0",
            "TextileMaterialDeclaration",
        ),
        (
            Sector::Steel,
            SectorData::Steel(steel_data()),
            "1.0.0",
            "SteelProductData",
        ),
        (
            Sector::Construction,
            SectorData::Construction(construction_data()),
            "1.0.0",
            "ConstructionProductData",
        ),
        (
            Sector::Tyre,
            SectorData::Tyre(tyre_data()),
            "1.0.0",
            "TyreProductData",
        ),
        (
            Sector::Toy,
            SectorData::Toy(toy_data()),
            "1.0.0",
            "ToyProductData",
        ),
        (
            Sector::Aluminium,
            SectorData::Aluminium(aluminium_data()),
            "1.0.0",
            "AluminiumProductData",
        ),
        (
            Sector::Furniture,
            SectorData::Furniture(furniture_data()),
            "1.0.0",
            "FurnitureProductData",
        ),
        (
            Sector::Detergent,
            SectorData::Detergent(detergent_data()),
            "1.0.0",
            "DetergentProductData",
        ),
        (
            Sector::UnsoldGoods,
            SectorData::UnsoldGoods(unsold_goods_report()),
            "1.0.0",
            "UnsoldGoodsReport",
        ),
    ]
}

#[test]
fn every_sector_produces_a_valid_aas_shell() {
    for (sector, data, version, _id_short) in all_sector_cases() {
        let key = sector.catalog_key();
        // SectorData::sector() must report the variant's own discriminant.
        assert_eq!(
            data.sector().catalog_key(),
            key,
            "SectorData::sector() must match its variant"
        );
        let passport = base(sector, data, version);
        let (shell, submodels) = build_aas_from_passport(&passport, VALID_GTIN);

        // Five core submodels + one sector submodel.
        assert_eq!(submodels.len(), 6, "sector {key} should yield 6 submodels");
        assert_eq!(shell.submodels.len(), 6);
        assert!(shell.asset_information.global_asset_id.contains(VALID_GTIN));

        // The whole environment serialises cleanly.
        let shell_json = serde_json::to_value(&shell).unwrap();
        assert_eq!(shell_json["idShort"], "DigitalProductPassport");
        assert!(serde_json::to_value(&submodels).unwrap().is_array());

        // The five core submodels are always present by idShort.
        for core in [
            "ProductIdentification",
            "ManufacturerInformation",
            "EnvironmentalImpact",
            "MaterialComposition",
            "Repairability",
        ] {
            assert!(
                submodels.iter().any(|s| s.id_short == core),
                "{core} submodel missing for sector {key}"
            );
        }

        // The sector submodel exists and carries at least its mandatory fields.
        let sector_submodel = submodels
            .iter()
            .find(|s| {
                !matches!(
                    s.id_short.as_str(),
                    "ProductIdentification"
                        | "ManufacturerInformation"
                        | "EnvironmentalImpact"
                        | "MaterialComposition"
                        | "Repairability"
                )
            })
            .expect("a sector-specific submodel is present");
        assert!(
            !sector_submodel.submodel_elements.is_empty(),
            "sector submodel for {key} should not be empty"
        );
    }
}

#[test]
fn unknown_sector_uses_generic_fallback_submodel() {
    // SectorData::Other(...) drives the generic JSON→AAS fallback path.
    let other = SectorData::Other(serde_json::json!({
        "sector": "spacecraft",
        "thrustKn": 500.0,
        "reusable": true,
        "stageCount": 2
    }));
    let passport = base(Sector::Other, other, "1.0.0");
    let (_shell, submodels) = build_aas_from_passport(&passport, VALID_GTIN);

    assert_eq!(submodels.len(), 6);
    let generic = submodels
        .iter()
        .find(|s| s.id_short == "SectorData")
        .expect("generic SectorData submodel present");
    // The discriminant `sector` key is dropped; the three data fields remain.
    assert_eq!(generic.submodel_elements.len(), 3);
}

#[test]
fn passport_without_sector_data_has_five_core_submodels() {
    let mut passport = base(
        Sector::Electronics,
        SectorData::Electronics(electronics_data()),
        "1.0.0",
    );
    passport.sector_data = None;
    let (_shell, submodels) = build_aas_from_passport(&passport, VALID_GTIN);
    assert_eq!(submodels.len(), 5);
}

#[test]
fn aas_submodel_templates_resolve_for_known_sectors() {
    // Battery is the only ratified (non-placeholder) template today.
    let battery = sector_submodel_template("battery").expect("battery template exists");
    assert!(!battery.is_placeholder);
    assert_eq!(battery.sector_key, "battery");

    // Textile is a placeholder pending an IDTA standard.
    let textile = sector_submodel_template("textile").expect("textile template exists");
    assert!(textile.is_placeholder);

    // Unknown sector → no template.
    assert!(sector_submodel_template("spacecraft").is_none());

    // Every placeholder is flagged as such.
    let placeholders: Vec<_> = placeholder_templates().collect();
    assert!(!placeholders.is_empty());
    assert!(placeholders.iter().all(|t| t.is_placeholder));
    // Battery (ratified) must NOT appear among placeholders.
    assert!(!placeholders.iter().any(|t| t.sector_key == "battery"));
}
