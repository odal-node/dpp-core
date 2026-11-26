//! The unsold-goods disclosure submodel.

use super::super::*;
use super::common::*;
use dpp_domain::Audience;
use dpp_domain::identifier::ProductIdentifier;
use dpp_domain::{Gtin, ProductGroup, ProductGroupData, ProductionRoute, SteelData};

/// The export is one shell and one `UnsoldGoods` submodel carrying the Annex I rows.
#[test]
fn build_aas_unsold_goods_produces_the_disclosure_submodel() {
    let environment = build_unsold_goods_environment(
        &sample_unsold_goods_report(),
        "assetKey",
        "urn:odal-node:test-asset:1",
    )
    .expect("a valid caller-chosen asset key");
    let sub = environment
        .submodels
        .iter()
        .find(|s| s.id_short == "UnsoldGoods");
    assert!(sub.is_some(), "UnsoldGoods submodel missing");

    let id_shorts: Vec<&str> = sub
        .unwrap()
        .submodel_elements
        .iter()
        .filter_map(|e| match e {
            AasSubmodelElement::Property(p) => Some(p.id_short.as_str()),
            _ => None,
        })
        .collect();

    // The Annex I header, the flattened line, and the two narrative rows.
    for expected in [
        "entityName",
        "financialYearStart",
        "financialYearEnd",
        "totalWeightKg",
        "line0CnCategories",
        "line0TotalDestructionPct",
        "measuresTaken",
        "measuresPlanned",
    ] {
        assert!(
            id_shorts.contains(&expected),
            "{expected} missing from {id_shorts:?}"
        );
    }
}

/// A line with no derogation projects no `Reason` element: an empty string
/// would read as a reason that is blank, and a made-up word as one that exists.
#[test]
fn a_line_with_no_derogation_projects_no_reason_element() {
    let mut report = sample_unsold_goods_report();
    report.lines[0].reason = None;
    let environment =
        build_unsold_goods_environment(&report, "assetKey", "urn:odal-node:test-asset:1")
            .expect("a valid caller-chosen asset key");
    let sub = environment
        .submodels
        .iter()
        .find(|s| s.id_short == "UnsoldGoods")
        .expect("UnsoldGoods submodel");

    let id_shorts: Vec<&str> = sub
        .submodel_elements
        .iter()
        .filter_map(|e| match e {
            AasSubmodelElement::Property(p) => Some(p.id_short.as_str()),
            _ => None,
        })
        .collect();

    assert!(id_shorts.contains(&"line0Description"));
    assert!(!id_shorts.contains(&"line0Reason"), "{id_shorts:?}");
}

/// A product group whose typed mapper was removed still ships its data — the
/// projection loses its *shape*, not its *content*.
///
/// This is the property that makes removing a typed lane safe rather than
/// lossy: the generic builder renders the variant's serialised fields, minus
/// the `product_group` discriminant, so a consumer still receives every value. What it
/// no longer receives is a submodel named for a template that no standards body
/// has ratified, which was never a claim we could support.
#[test]
fn a_product_group_without_a_typed_mapper_still_carries_its_data() {
    let mut passport = minimal_passport(ProductGroup::Steel);
    passport.product_group_data = Some(ProductGroupData::Steel(SteelData {
        product_identifier: ProductIdentifier::gs1(
            Gtin::parse("09506000134352").expect("valid gtin"),
        ),
        co2e_per_tonne_steel: 1.8,
        recycled_scrap_content_pct: 62.0,
        product_category: "flat".into(),
        country_of_origin: "DE".into(),
        production_route: ProductionRoute::ElectricArc,
        annual_production_tonnes: None,
    }));

    let (_, submodels) =
        build_aas_from_passport(&passport, identity_of(&passport), Audience::Public)
            .expect("masking");
    let product_group_submodel = submodels
        .iter()
        .find(|s| s.id_short == "ProductGroupData")
        .expect("a provisional product_group renders through the generic builder");

    assert!(
        !product_group_submodel.submodel_elements.is_empty(),
        "the generic projection must carry the product_group's fields, not drop them"
    );
    assert!(
        product_group_submodel.semantic_id.is_none(),
        "a generic projection asserts no semanticId — there is no ratified \
         template for it to name"
    );
}

// ─── Environment ──────────────────────────────────────────────────────────────
