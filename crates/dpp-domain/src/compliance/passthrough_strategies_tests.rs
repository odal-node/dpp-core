//! What the two Apache-2.0 passthrough strategies do and do not assert.

use super::passthrough_strategies::*;
use crate::domain::compliance::ComplianceErrorKind;
use crate::domain::product_group::ProductGroupData;
use crate::ports::compliance::ComplianceStatus;
use crate::ports::compliance::ComplianceStrategy;

fn battery() -> ProductGroupData {
    ProductGroupData::Battery(Box::new(crate::test_support::sample_battery_data()))
}

fn textile() -> ProductGroupData {
    ProductGroupData::Textile(Box::new(crate::domain::product_group::TextileData {
        carbon_footprint_kg_co2e: Some(8.5),
        recycled_content_pct: Some(42.0),
        repair_score: Some(7.5),
        ..crate::test_support::sample_textile_data()
    }))
}

/// Passthrough carries the manufacturer's numbers and adds nothing.
#[test]
fn textile_passthrough_lifts_declared_metrics_verbatim() {
    let result = PassthroughTextileStrategy
        .compute(&textile(), None)
        .expect("textile data");
    assert_eq!(result.co2e_score, Some(8.5));
    assert_eq!(result.recycled_content_pct, Some(42.0));
    assert_eq!(result.repairability_index, Some(7.5));
    assert_eq!(
        result.compliance_status,
        ComplianceStatus::PassthroughNoValidation
    );
    assert!(result.violations.is_empty() && result.warnings.is_empty());
    assert!(
        result.receipt.is_none() && result.ruleset_version.is_none(),
        "no calculation ran, so there is no receipt to show for one"
    );
}

/// The battery strategy declines to invent a single recycled-content figure.
///
/// Art. 8(2)/8(3) set per-metal minima over two different measurement bases.
/// Collapsing four into one would be arithmetically clean and regulatorily
/// meaningless, and a reader would take the number for a compliance figure.
#[test]
fn battery_passthrough_leaves_recycled_content_unset() {
    let result = PassthroughBatteryStrategy
        .compute(&battery(), None)
        .expect("battery data");
    assert!(
        result.recycled_content_pct.is_none(),
        "four per-metal percentages must not be collapsed into one"
    );
    assert!(
        result.repairability_index.is_none(),
        "no EU repairability index applies to batteries"
    );
    assert_eq!(
        result.compliance_status,
        ComplianceStatus::PassthroughNoValidation
    );
}

/// A strategy handed another product group's data errors rather than panicking.
///
/// This is a host routing bug, and a library that aborts the process on one
/// gives the host no way to report it.
#[test]
fn a_strategy_refuses_another_product_groups_data() {
    let err = PassthroughBatteryStrategy
        .compute(&textile(), None)
        .expect_err("battery strategy must refuse textile data");
    assert_eq!(err.kind, ComplianceErrorKind::InvalidInput);
    assert!(err.message.contains("textile"), "{}", err.message);
}

/// Each strategy answers with the catalog key it is registered under.
///
/// The registry keys its map on this, so a wrong answer here silently routes
/// every passport of that product group to the fallback.
#[test]
fn product_group_keys_match_the_catalog() {
    let catalog = crate::ProductGroupCatalog::new();
    for key in [
        PassthroughBatteryStrategy.product_group_key(),
        PassthroughTextileStrategy.product_group_key(),
    ] {
        assert!(
            catalog.get(key).is_some(),
            "'{key}' is not a catalog product_group key"
        );
    }
}
