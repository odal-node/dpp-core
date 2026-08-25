//! Dispatch through the strategy trait, and the bare-passthrough fallback.

use super::passthrough_registry::*;
use crate::domain::compliance::{ComplianceError, ComplianceResult};
use crate::domain::product_group::{
    BatteryData, FibreEntry, ProductGroup, ProductGroupData, TextileData,
};
use crate::ports::compliance::ComplianceStatus;
use crate::ports::compliance::{ComplianceRegistry, ComplianceStrategy};
use chrono::NaiveDate;

fn battery_data() -> ProductGroupData {
    ProductGroupData::Battery(Box::new(BatteryData {
        recycled_content_lithium_pct: Some(12.5),
        rated_capacity_kwh: Some(32.0),
        ..crate::test_support::sample_battery_data()
    }))
}

fn textile_data() -> ProductGroupData {
    ProductGroupData::Textile(Box::new(TextileData {
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
    }))
}

/// No product group gets a determination, whether or not it has a strategy.
///
/// "Determination" is the *status* and the findings — that is what a
/// notified body reads and what blocks a publish. Metrics are not a
/// determination: `ComplianceResult::co2e_score` is documented as
/// "calculated **or manufacturer-supplied**", and carrying a declared value
/// under `PassthroughNoValidation` claims nothing about it.
///
/// This test previously also asserted the metrics were `None`, which was
/// true of the stub rather than of passthrough. Lifting a declared metric is
/// exactly what "stores manufacturer-supplied values verbatim" means; the
/// invariant that must not move is the one asserted here.
#[test]
fn passthrough_makes_no_determination_for_any_product_group() {
    let registry = PassthroughRegistry::new();
    for (product_group, data) in [
        (ProductGroup::Battery, battery_data()),
        (ProductGroup::Textile, textile_data()),
        // A product group with no per-product group handling used to return NotImplemented;
        // now it takes the bare-passthrough fallback.
        (ProductGroup::Electronics, battery_data()),
    ] {
        let result = registry
            .compute(product_group.catalog_key(), &data, None)
            .unwrap();
        assert_eq!(
            result.compliance_status,
            ComplianceStatus::PassthroughNoValidation,
            "{product_group:?} must not receive a determination"
        );
        assert!(
            result.violations.is_empty() && result.warnings.is_empty(),
            "{product_group:?} passthrough must produce no findings"
        );
        assert!(
            result.receipt.is_none() && result.ruleset_version.is_none(),
            "{product_group:?} ran no calculation, so it has no receipt to show for one"
        );
    }
}

/// A registered product group routes through its strategy; an unregistered one does
/// not error.
///
/// The catalog is open by design — a product group can be added as manifest
/// plus schema with no Rust — so a product group without a strategy must still be
/// served. `UnknownProductGroup` here would make this registry the one closed part
/// of a data-driven model.
#[test]
fn a_registered_product_group_uses_its_strategy_and_the_rest_fall_back() {
    let registry = PassthroughRegistry::new();
    assert_eq!(
        registry.registered_product_groups(),
        vec!["battery", "textile"]
    );

    // Textile has a strategy: its declared metrics are lifted.
    let textile = registry.compute("textile", &textile_data(), None).unwrap();
    assert_eq!(textile.co2e_score, Some(8.5));
    assert_eq!(textile.recycled_content_pct, Some(30.0));
    assert_eq!(textile.repairability_index, Some(7.5));

    // Electronics has none: served, with nothing lifted.
    let electronics = registry
        .compute("electronics", &battery_data(), None)
        .unwrap();
    assert_eq!(electronics.co2e_score, None);
    assert_eq!(
        electronics.compliance_status,
        ComplianceStatus::PassthroughNoValidation
    );

    // A product group this build has never heard of is served too.
    let unknown = registry.compute("quantum-widget", &battery_data(), None);
    assert!(
        unknown.is_ok(),
        "an unmodelled product_group must not be an error here"
    );
}

/// `register` replaces, so a tier can substitute one product group's behaviour.
///
/// This is the whole point of the per-product group seam. A silent refusal would
/// leave the host running passthrough while believing it had swapped in its
/// own strategy.
#[test]
fn registering_a_strategy_replaces_the_one_already_there() {
    struct AlwaysFortyTwo;
    impl ComplianceStrategy for AlwaysFortyTwo {
        fn product_group_key(&self) -> &str {
            "textile"
        }
        fn compute(
            &self,
            _: &ProductGroupData,
            _: Option<NaiveDate>,
        ) -> Result<ComplianceResult, ComplianceError> {
            Ok(ComplianceResult {
                co2e_score: Some(42.0),
                ..ComplianceResult::passthrough()
            })
        }
    }

    let mut registry = PassthroughRegistry::new();
    registry.register(Box::new(AlwaysFortyTwo));
    assert_eq!(
        registry
            .compute("textile", &textile_data(), None)
            .unwrap()
            .co2e_score,
        Some(42.0),
        "the registered strategy must displace the built-in one"
    );
    assert_eq!(
        registry.registered_product_groups(),
        vec!["battery", "textile"],
        "replacing must not add a second entry for the same product_group"
    );
}

/// `empty()` registers nothing, so every product group takes the fallback.
#[test]
fn an_empty_registry_falls_back_for_everything() {
    let registry = PassthroughRegistry::empty();
    assert!(registry.registered_product_groups().is_empty());
    let result = registry.compute("textile", &textile_data(), None).unwrap();
    assert_eq!(result.co2e_score, None);
    assert_eq!(
        result.compliance_status,
        ComplianceStatus::PassthroughNoValidation
    );
}
