//! Per-product-group parity: every Rust type round-trips through the current
//! JSON schema its manifest names, and reports the category it actually states.

use super::*;
use crate::catalog::ProductGroupCatalog;
use crate::identifier::Gtin;
use crate::schemas::VersionedSchemaRegistry;

use super::tests::{minimal_battery_data, test_textile_data};

// One schema-valid instance per product group, constructed through the *Rust type*
// (not a hand-written JSON literal) and round-tripped through that product group's
// own current embedded schema via `validate_strict` — the same fail-closed
// call the publish path uses. This is the test class that would have caught
// `BatteryType::Sli` serialising to `"sli"` against a schema expecting
// `"starting-lighting-ignition"`: a serde rename and the schema it targets
// can drift independently, and only a value built from the Rust type (so its
// wire shape is whatever serde actually emits today) catches that drift.

fn sample_steel_data() -> ProductGroupData {
    ProductGroupData::Steel(SteelData {
        gtin: Gtin::parse("09506000134352").unwrap(),
        co2e_per_tonne_steel: 1.8,
        recycled_scrap_content_pct: 35.0,
        product_category: "flat".into(),
        country_of_origin: "DE".into(),
        production_route: ProductionRoute::ElectricArc,
        annual_production_tonnes: None,
    })
}

fn sample_aluminium_data() -> ProductGroupData {
    ProductGroupData::Aluminium(AluminiumData {
        gtin: Gtin::parse("09506000134352").unwrap(),
        alloy_grade: "6xxx".into(),
        production_route: ProductionRoute::SecondaryRecycled,
        co2e_per_tonne_kg: 1200.0,
        recycled_content_pct: 60.0,
        country_of_origin: "DE".into(),
        annual_production_tonnes: None,
    })
}

fn sample_electronics_data() -> ProductGroupData {
    ProductGroupData::Electronics(ElectronicsData {
        gtin: Gtin::parse("09506000134352").unwrap(),
        product_category: DeviceType::Smartphone,
        energy_efficiency_class: EnergyEfficiencyClass::B,
        co2e_per_unit_kg: 120.0,
        repairability_score: Some(RepairabilityScore {
            overall: 7.5,
            criteria: vec![RepairCriterion {
                name: "spare-parts-availability".into(),
                score: 8.0,
                weight: 0.5,
            }],
        }),
        spare_parts_available: Some(true),
        repair_manual_url: None,
        disassembly_instructions_url: None,
        svhc_substances: None,
        rohs_compliant: Some(true),
        critical_raw_materials: None,
        recycled_content_pct: None,
        standby_power_w: None,
        expected_lifetime_years: None,
        firmware_update_until: None,
    })
}

fn sample_construction_data() -> ProductGroupData {
    ProductGroupData::Construction(ConstructionData {
        gtin: Gtin::parse("09506000134352").unwrap(),
        product_family: "cement".into(),
        country_of_origin: "DE".into(),
        co2e_per_functional_unit_kg: 0.8,
        functional_unit: "per tonne".into(),
        recycled_content_pct: None,
        epd_url: None,
        ce_marking: Some(true),
    })
}

fn sample_tyre_data() -> ProductGroupData {
    ProductGroupData::Tyre(TyreData {
        gtin: Gtin::parse("09506000134352").unwrap(),
        tyre_class: "C1".into(),
        fuel_efficiency_class: "B".into(),
        wet_grip_class: "A".into(),
        external_rolling_noise_db: 68.0,
        noise_performance_class: None,
        rolling_resistance_n_per_kn: None,
        recycled_rubber_pct: None,
        co2e_per_tyre_kg: None,
    })
}

fn sample_toy_data() -> ProductGroupData {
    ProductGroupData::Toy(ToyData {
        gtin: Gtin::parse("09506000134352").unwrap(),
        age_group: "3-6".into(),
        primary_material: "wood".into(),
        ce_marking: true,
        country_of_origin: "DE".into(),
        svhc_substances: None,
        contains_battery: Some(false),
        repairability_info: None,
    })
}

fn sample_furniture_data() -> ProductGroupData {
    ProductGroupData::Furniture(FurnitureData {
        gtin: Gtin::parse("09506000134352").unwrap(),
        product_type: "chair".into(),
        primary_material: "solid-wood".into(),
        country_of_origin: "DE".into(),
        co2e_per_unit_kg: None,
        recycled_content_pct: None,
        repairability_score: Some(7.0),
        svhc_substances: None,
        disassembly_instructions_url: None,
        end_of_life_instructions: None,
    })
}

fn sample_detergent_data() -> ProductGroupData {
    ProductGroupData::Detergent(DetergentData {
        gtin: Gtin::parse("09506000134352").unwrap(),
        product_type: "laundry".into(),
        format: "liquid".into(),
        surfactants: vec![SurfactantEntry {
            name: "Sodium Laureth Sulfate".into(),
            biodegradable: true,
            concentration_band: "5-15%".into(),
            cas_number: None,
        }],
        country_of_origin: "DE".into(),
        co2e_per_unit_kg: None,
        packaging_recyclable: None,
        recommended_dosage_ml: None,
        biodegradable: None,
    })
}

fn sample_mattress_data() -> ProductGroupData {
    ProductGroupData::Mattress(MattressData {
        gtin: Gtin::parse("09506000134352").unwrap(),
        primary_material: "upholstered".into(),
        country_of_origin: "DE".into(),
        co2e_per_unit_kg: None,
        recycled_content_pct: None,
        repairability_score: None,
        svhc_substances: None,
        disassembly_instructions_url: None,
        end_of_life_instructions: None,
    })
}

fn sample_unsold_goods_data() -> ProductGroupData {
    ProductGroupData::UnsoldGoods(crate::test_support::sample_unsold_goods_report())
}

#[test]
fn every_product_group_with_an_embedded_schema_round_trips_through_its_current_schema() {
    let catalog = ProductGroupCatalog::new();
    let registry = VersionedSchemaRegistry::new();

    let samples: Vec<ProductGroupData> = vec![
        minimal_battery_data(),
        ProductGroupData::Textile(Box::new(test_textile_data())),
        sample_unsold_goods_data(),
        sample_steel_data(),
        sample_electronics_data(),
        sample_construction_data(),
        sample_tyre_data(),
        sample_toy_data(),
        sample_aluminium_data(),
        sample_furniture_data(),
        sample_mattress_data(),
        sample_detergent_data(),
    ];
    // Every non-Other ProductGroupData variant must be exercised — a future product group
    // added to the enum without a sample here would silently skip this gate.
    //
    // Mattress was the case this guard was meant to catch and did not: it has
    // an embedded schema and a variant, and the count was carried at 11 rather
    // than the sample being added, so the group this test is named for was the
    // one group it skipped.
    assert_eq!(
        samples.len(),
        12,
        "expected one sample per non-Other product_group"
    );

    for sample in samples {
        let product_group = sample.product_group();
        let key = product_group.catalog_key();
        let Some(version) = catalog.resolve_schema_version(key, None) else {
            panic!("product_group '{key}' has no catalog entry — add one or exclude it here");
        };
        let mut json = serde_json::to_value(&sample).expect("serialize product_group data");
        // ProductGroupData is internally tagged (`#[serde(tag = "productGroup")]`); schemas
        // validate the inner object with `additionalProperties:false`, so
        // strip the tag (mirrors `validate_against_schema` in create.rs).
        json.as_object_mut().unwrap().remove("productGroup");

        registry
            .validate_strict(key, &version, &json)
            .unwrap_or_else(|e| {
                panic!(
                    "product_group '{key}' v{version}: a Rust-type-constructed valid \
                     instance failed its own embedded schema — type/schema \
                     drift: {e:?}"
                )
            });
    }
}

/// What each product group answers for its category, and the reason it is that
/// field rather than another one.
///
/// A table rather than an assertion per group, because the interesting failure
/// is a *changed* answer: seven groups name their category axis seven different
/// things, so reading the wrong field compiles, type-checks, and returns a
/// perfectly plausible string. Pinning all twelve in one place makes such a
/// change show up as an edit to this list.
const EXPECTED_CATEGORY: &[(&str, Option<&str>)] = &[
    // Answers — the group's own category axis, whatever its act calls it.
    ("battery", Some("portable")),
    ("construction", Some("cement")),
    ("detergent", Some("laundry")),
    ("electronics", Some("smartphone")),
    ("furniture", Some("chair")),
    ("steel", Some("flat")),
    ("tyre", Some("C1")),
    // Declines — see each group's `product_category` for why. These are not
    // gaps to be filled in later by whoever touches the file next: two of them
    // record that the catalog is wrong, and three that the group has no
    // category at all.
    ("aluminium", None),
    ("mattress", None),
    ("textile", None),
    ("toy", None),
    ("unsoldGoods", None),
];

#[test]
fn every_product_group_reports_the_category_it_states() {
    let samples: Vec<ProductGroupData> = vec![
        minimal_battery_data(),
        ProductGroupData::Textile(Box::new(test_textile_data())),
        sample_unsold_goods_data(),
        sample_steel_data(),
        sample_electronics_data(),
        sample_construction_data(),
        sample_tyre_data(),
        sample_toy_data(),
        sample_aluminium_data(),
        sample_furniture_data(),
        sample_mattress_data(),
        sample_detergent_data(),
    ];
    assert_eq!(
        samples.len(),
        EXPECTED_CATEGORY.len(),
        "every non-Other product group needs a row in EXPECTED_CATEGORY"
    );

    for sample in samples {
        let tag = sample.product_group();
        let tag = tag.wire_str();
        let (_, expected) = EXPECTED_CATEGORY
            .iter()
            .find(|(group, _)| *group == tag)
            .unwrap_or_else(|| panic!("product group '{tag}' has no row in EXPECTED_CATEGORY"));

        assert_eq!(
            sample.product_category(),
            *expected,
            "product group '{tag}' reports a different category than this table pins"
        );

        // The answer has to be something the record *says*, not a constant the
        // accessor holds. Checking it against the serialised form is what
        // separates "reads the category field" from "returns a string that
        // happens to match the fixture" — and it is the wire form, so an enum
        // whose `wire_str` drifted from its serde tag fails here too.
        if let Some(category) = *expected {
            let json = serde_json::to_value(&sample).expect("serialize product group data");
            let stated = json
                .as_object()
                .expect("product group data serialises to an object")
                .values()
                .any(|v| v.as_str() == Some(category));
            assert!(
                stated,
                "product group '{tag}' reports category '{category}', which appears \
                 nowhere in the record it serialises to: {json}"
            );
        }
    }
}

/// An untyped payload cannot answer, and must not pretend to.
///
/// `Other` holds a product group this build has no variant for, so nothing here
/// knows which of its keys is a category — the same reason it answers no GTIN
/// and no model identifier. A caller enforcing scope has to read this as
/// *unevaluable*, never as unscoped.
#[test]
fn an_untyped_product_group_states_no_category() {
    let data = ProductGroupData::other(serde_json::json!({
        "productGroup": "packaging",
        "productCategory": "corrugated-board",
    }))
    .expect("an unknown tag builds an Other payload");

    assert_eq!(data.product_category(), None);
}
