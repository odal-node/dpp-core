//! Projection of product-group fields.

use super::super::*;
use super::common::*;
use crate::identity::AssetIdentity;
use dpp_domain::Audience;
use dpp_domain::{CarbonFootprintClass, HazardSymbol, Passport, ProductGroup, ProductGroupData};
use serde_json::json;

/// A product group-data key the passport's declared schema version does not know about
/// is dropped, for every audience.
///
/// Disclosure is sourced from the declared version, so an undeclared key is
/// classified by nobody and would fall to the policy default — `Public`. That
/// makes the version-sourced policy *less* safe than the catalog map in exactly
/// one direction, and this is the guard that closes it.
///
/// Reaching this needs an invalid passport: every product group schema sets
/// `additionalProperties: false`, so validation already rejects such a
/// document. Defence in depth, and asserted for the credentialed audience too —
/// an unclassified field is not more disclosable to a repairer than to anyone
/// else.
#[test]
fn a_product_group_field_the_declared_version_does_not_know_is_dropped() {
    let mut passport = minimal_passport(ProductGroup::Battery);
    passport.product_group_data = Some(ProductGroupData::Battery(Box::new(
        battery_data_with_due_diligence(),
    )));

    let mut document = serde_json::to_value(&passport).expect("serialises");
    document["productGroupData"]["smuggledField"] = json!("should not survive");
    let smuggled: Passport = serde_json::from_value(document).expect("round-trips via Other keys");

    for audience in [
        Audience::Public,
        Audience::LegitimateInterest,
        Audience::Authority,
    ] {
        let (_, submodels) = build_aas_from_passport(
            &smuggled,
            AssetIdentity::from_passport(&smuggled).expect("the fixture carries an identifier"),
            audience,
        )
        .expect("masking succeeds");
        let serialised = serde_json::to_string(&submodels).expect("serialises");
        assert!(
            !serialised.contains("smuggledField"),
            "{audience:?}: an unclassified product_group field reached the projection"
        );
    }
}

/// A carbon footprint class never projects without the ruleset that produced it.
///
/// `BatteryData::carbon_footprint_class` says of itself that it is "meaningless
/// without the two provenance fields below: the same label denotes different
/// thresholds under different revisions of the scale, and Art. 7(2) requires
/// those thresholds to be reviewed every three years". The mapper emitted the
/// bare label and neither provenance field, so an exported AAS carried a class
/// a consumer had no way to interpret — not a weaker claim than a qualified
/// one, an unfalsifiable one.
///
/// The passport fixtures are all-`None` for these fields, which is why the
/// committed-Environment test never caught it: an unpopulated field cannot
/// reveal a mapping that does not exist.
#[test]
fn a_carbon_footprint_class_projects_with_its_ruleset() {
    let mut data = battery_data_with_due_diligence();
    data.carbon_footprint_class = Some(CarbonFootprintClass::new("B").expect("valid label"));
    data.carbon_footprint_class_ruleset_id = Some("eu-battery-cfb".into());
    data.carbon_footprint_class_ruleset_version = Some("2026.1".into());

    let mut passport = minimal_passport(ProductGroup::Battery);
    passport.product_group_data = Some(ProductGroupData::Battery(Box::new(data)));
    let (_, submodels) =
        build_aas_from_passport(&passport, identity_of(&passport), Audience::Public)
            .expect("masking");
    let json = serde_json::to_string(&submodels).expect("serialises");

    assert!(
        json.contains("carbonFootprintClass"),
        "the class must project"
    );
    assert!(
        json.contains("carbonFootprintClassRulesetId"),
        "and never without the ruleset that produced it"
    );
    assert!(
        json.contains("carbonFootprintClassRulesetVersion"),
        "including the revision, since the scale is reviewed every three years"
    );
}

/// The Annex XIII point 1 conditions and Annex VI Part A safety data reach the
/// projection, not just the passport.
///
/// Each of these is a property of the asset rather than a document about it, so
/// none falls under the document-shaped exclusion the mapper states. They were
/// absent purely because nobody had mapped them, and the committed-Environment
/// fixture could not show it: its battery data leaves every one of them `None`.
#[test]
fn the_point_1_conditions_and_safety_data_project() {
    let mut data = battery_data_with_due_diligence();
    data.usable_extinguishing_agent = Some("Class D dry powder".into());
    data.hazard_symbol = Some(HazardSymbol::Cadmium);
    data.commercial_warranty_period_months = Some(96);
    data.recycled_content_reporting_year = Some(2026);
    data.expected_lifetime_reference_test = Some("IEC 62660-1:2018".into());
    // Both types are `#[non_exhaustive]`, so they are built through serde
    // rather than a struct literal — which incidentally pins their wire names.
    data.voltage_temperature_range =
        Some(serde_json::from_value(json!({ "minC": -20.0, "maxC": 60.0 })).expect("range"));
    data.hazardous_substances = Some(vec![
        serde_json::from_value(json!({
            "name": "Nickel sulfate",
            "casNumber": "7786-81-4",
            "concentrationPct": 0.4
        }))
        .expect("substance"),
    ]);
    data.battery_model_id = Some("LFP-64-A".into());
    data.manufacturing_place = Some("PL:Wroclaw".into());

    let mut passport = minimal_passport(ProductGroup::Battery);
    passport.product_group_data = Some(ProductGroupData::Battery(Box::new(data)));
    let (_, submodels) =
        build_aas_from_passport(&passport, identity_of(&passport), Audience::Public)
            .expect("masking");
    let json = serde_json::to_string(&submodels).expect("serialises");

    for id in [
        "usableExtinguishingAgent",
        "hazardSymbol",
        "commercialWarrantyPeriodMonths",
        "recycledContentReportingYear",
        "expectedLifetimeReferenceTest",
        "voltageTemperatureRange",
        "hazardousSubstances",
        "batteryModelId",
        "manufacturingPlace",
    ] {
        assert!(json.contains(id), "{id} is public and must project");
    }
    // The range is a collection of two numbers, not two loose properties.
    assert!(json.contains("minC") && json.contains("maxC"));
    // Nickel sulfate is Annex VI Part A point 8 content and public.
    assert!(json.contains("7786-81-4"));
}
