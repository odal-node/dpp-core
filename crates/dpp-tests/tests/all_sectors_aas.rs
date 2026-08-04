//! Cross-crate integration test: AAS mapping across every sector.
//!
//! `build_aas_from_passport` (dpp-aas) is the primary AAS entry
//! point — it dispatches each sector's `SectorData` to a dedicated submodel
//! mapper. This test feeds a fully-populated passport for **every** sector
//! through that path (so each mapper's optional-field branches execute) and
//! asserts the resulting shell + submodels are well-formed and serialisable.
//!
//! It complements the per-sector E2E tests (battery, textile) by guaranteeing
//! no sector mapper silently regresses or panics, and that the AAS submodel
//! template registry stays in sync with the sectors.

use chrono::{DateTime, Utc};
use dpp_aas::{
    build_aas_environment, build_aas_from_passport, map_dpp_to_aas_submodel, placeholder_templates,
    sector_submodel_template,
};
use dpp_domain::Audience;
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
        // Fixed rather than `Utc::now()`: this value reaches the committed
        // Environment fixtures, and a clock reading there would make them differ
        // on every run.
        firmware_update_until: Some(
            "2031-06-30T00:00:00Z"
                .parse::<DateTime<Utc>>()
                .expect("a date"),
        ),
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
        let key = sector.catalog_key().to_owned();
        // SectorData::sector() must report the variant's own discriminant.
        assert_eq!(
            data.sector().catalog_key(),
            key,
            "SectorData::sector() must match its variant"
        );
        let passport = base(sector, data, version);
        let (shell, submodels) =
            build_aas_from_passport(&passport, VALID_GTIN, Audience::Public).expect("masking");

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

/// An unrecognised sector's fields never reach the AAS output, for any audience.
///
/// The catalog has no descriptor for such a sector, so no field of it is
/// classified, so every field of it is unclassified — and both policies the
/// masking seam can pick default unclassified fields to `Public`. Left to the
/// filter alone, the whole payload passes through. The builder therefore applies
/// a structural backstop: with no field policy, keep only the discriminant.
///
/// This test used to assert the opposite — that all three `spacecraft` fields
/// reached the output. That was the library faithfully reporting a leak nobody
/// had noticed, because the platform's two public doors each carry this backstop
/// and the library, which is what a third party actually depends on, did not.
#[test]
fn unknown_sector_fields_never_reach_the_aas_output() {
    let spacecraft = || {
        SectorData::other(serde_json::json!({
            "sector": "spacecraft",
            "thrustKn": 500.0,
            "reusable": true,
            "stageCount": 2
        }))
        .expect("spacecraft has no typed variant")
    };

    // Every audience, not just Public: an unmodelled sector has no field policy
    // for any of them, so a credentialed reader must not receive more.
    for audience in [
        Audience::Public,
        Audience::LegitimateInterest,
        Audience::Authority,
    ] {
        let passport = base(Sector::Other("spacecraft".into()), spacecraft(), "1.0.0");
        let (_shell, submodels) = build_aas_from_passport(&passport, VALID_GTIN, audience)
            .expect("the redacted document still round-trips");

        assert_eq!(
            submodels.len(),
            6,
            "{audience:?}: the sector submodel is still present"
        );
        let generic = submodels
            .iter()
            .find(|s| s.id_short == "SectorData")
            .expect("generic SectorData submodel present");

        assert!(
            generic.submodel_elements.is_empty(),
            "{audience:?}: an unmodelled sector's fields reached the AAS output: {:?}",
            generic.submodel_elements
        );

        // Asserted over the serialised document too, so a field surfacing under
        // some other submodel or nesting depth is caught rather than missed by
        // looking only where it is expected not to be.
        let rendered = serde_json::to_string(&submodels).expect("serialises");
        for leaked in ["thrustKn", "reusable", "stageCount"] {
            assert!(
                !rendered.contains(leaked),
                "{audience:?}: '{leaked}' appears somewhere in the AAS output"
            );
        }
        // Not vacuous: the sector is still identified, via ProductIdentification.
        assert!(
            rendered.contains("spacecraft"),
            "{audience:?}: the sector is no longer identified at all"
        );
    }
}

/// A *known* sector is filtered by its policy, not blanket-redacted.
///
/// The backstop keys on "the catalog has no descriptor", so a bug that widened
/// that condition would empty every sector submodel and every masking assertion
/// above would still pass. This is the guard against that.
#[test]
fn a_known_sector_is_not_caught_by_the_unknown_sector_backstop() {
    let passport = base(
        Sector::Electronics,
        SectorData::Electronics(electronics_data()),
        "1.0.0",
    );
    let (_shell, submodels) =
        build_aas_from_passport(&passport, VALID_GTIN, Audience::Public).expect("masking");
    let sector_submodel = submodels
        .iter()
        .find(|s| s.id_short == "ElectronicsProductData")
        .expect("the typed electronics submodel is present");
    assert!(
        !sector_submodel.submodel_elements.is_empty(),
        "a catalogued sector was blanket-redacted by the unknown-sector backstop"
    );
}

#[test]
fn passport_without_sector_data_has_five_core_submodels() {
    let mut passport = base(
        Sector::Electronics,
        SectorData::Electronics(electronics_data()),
        "1.0.0",
    );
    passport.sector_data = None;
    let (_shell, submodels) =
        build_aas_from_passport(&passport, VALID_GTIN, Audience::Public).expect("masking");
    assert_eq!(submodels.len(), 5);
}

#[test]
fn aas_submodel_templates_resolve_for_known_sectors() {
    let battery = sector_submodel_template("battery").expect("battery template exists");
    assert_eq!(battery.sector_key, "battery");

    let textile = sector_submodel_template("textile").expect("textile template exists");
    assert!(textile.is_placeholder());

    // Unknown sector → no template.
    assert!(sector_submodel_template("spacecraft").is_none());

    let placeholders: Vec<_> = placeholder_templates().collect();
    assert!(!placeholders.is_empty());
    assert!(placeholders.iter().all(|t| t.is_placeholder()));
}

/// **Every** template is a placeholder today, and that is the honest state.
///
/// This test used to record the opposite for battery. It was wrong: battery's
/// identifier was reverted out of the Catena-X namespace into ours on
/// 2026-07-29 without the flag following, so the one template exempted from the
/// conformance gate was exempted on the strength of an identifier it no longer
/// carried.
///
/// Written as an explicit inventory rather than a loop over the derived flag,
/// because the derived flag would agree with itself no matter what the data
/// said. Adopting a genuine third-party identifier is what flips an entry here,
/// and that must be a visible diff in this list, not a silent change of state.
#[test]
fn no_sector_template_yet_names_a_ratified_third_party_standard() {
    let ratified: Vec<&str> = dpp_domain::SectorCatalog::new()
        .all()
        .iter()
        .filter_map(|d| sector_submodel_template(d.key.as_str()))
        .filter(|t| !t.is_placeholder())
        .map(|t| t.sector_key)
        .collect();

    assert!(
        ratified.is_empty(),
        "these templates claim a ratified third-party template: {ratified:?}. \
         Confirm a named reader verified each identifier against the authority's \
         own source and recorded it in semantic_ids/allowlist.json, then update \
         this test."
    );
}

/// A template's declared `version` must match the version its identifier ends
/// with.
///
/// The two are the same fact written twice, and they had already drifted: the
/// battery entry declared `"6.0.0"` — the Catena-X aspect model's version —
/// against an identifier ending `:1.0`, left behind when the identifier was
/// reverted. Duplication that nothing checks is how a stale value outlives the
/// thing it was copied from.
#[test]
fn a_template_version_matches_its_identifier() {
    for template in dpp_domain::SectorCatalog::new()
        .all()
        .iter()
        .filter_map(|d| sector_submodel_template(d.key.as_str()))
    {
        // Only our own identifiers carry the version as a trailing segment; a
        // third party's scheme is theirs to choose.
        if !template.is_placeholder() {
            continue;
        }
        assert!(
            template
                .semantic_id
                .ends_with(&format!(":{}", template.version)),
            "template '{}' declares version '{}' but its identifier is '{}'",
            template.sector_key,
            template.version,
            template.semantic_id
        );
    }
}

/// Every catalog sector must be exercised through the AAS path somewhere.
///
/// Without this, adding a sector to the catalog and forgetting to extend
/// `all_sector_cases` leaves that sector's AAS mapping untested — silently,
/// because the loop above simply iterates one case fewer.
#[test]
fn every_catalog_sector_has_an_aas_case() {
    // Battery is exercised by `battery_end_to_end.rs` rather than here, so it is
    // covered but not by this table. Any other absence is a real gap.
    const COVERED_ELSEWHERE: &[&str] = &["battery"];

    let catalog = dpp_domain::catalog::SectorCatalog::new();
    let covered: Vec<String> = all_sector_cases()
        .iter()
        .map(|(s, _, _, _)| s.catalog_key().to_owned())
        .collect();

    for d in catalog.all().iter() {
        let key = d.key.as_str();
        assert!(
            covered.iter().any(|c| c == key) || COVERED_ELSEWHERE.contains(&key),
            "catalog sector '{key}' has no case in all_sector_cases() — its AAS              mapping is untested"
        );
    }
}

// ─── semanticId provenance gate ──────────────────────────────────────────────
//
// A semanticId asserts, to a machine, that one of our fields means what a
// standards body says its identifier means — and nothing re-reads that claim
// once it is written. So it is enforced here rather than trusted to a comment.
//
// The rule: every identifier we emit is either in our own `urn:odal-node:`
// namespace, or carries a provenance record in `semantic_ids/allowlist.json`
// naming who verified it against the authority's own source, and when.

// Taken from the crate rather than restated, so the provenance gate and
// `SubmodelTemplate::is_placeholder` cannot disagree about where our namespace
// ends. A second copy here would be a third place for the boundary to drift.
use dpp_aas::semantic_ids::OWN_NAMESPACE;

/// The allowlist, minus any entry whose provenance is incomplete.
///
/// An entry missing `verifiedOn` or `verifiedBy` is dropped rather than
/// honoured, so a half-filled record fails the gate exactly like an absent one.
fn allowlisted_identifiers() -> Vec<String> {
    allowlisted_from(&allowlist_document())
}

fn allowlist_document() -> serde_json::Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../dpp-aas/src/semantic_ids/allowlist.json"
    );
    let raw = std::fs::read_to_string(path).expect("the allowlist file is present");
    serde_json::from_str(&raw).expect("the allowlist is valid JSON")
}

/// Split out from the loader so the provenance rule can be tested against
/// synthetic entries. With the live allowlist deliberately empty, testing it
/// through the real file would assert nothing.
fn allowlisted_from(doc: &serde_json::Value) -> Vec<String> {
    let entries = doc["allowlist"]
        .as_object()
        .expect("the allowlist has an `allowlist` object");

    entries
        .iter()
        .filter(|(_, record)| {
            let filled = |field: &str| {
                record[field]
                    .as_str()
                    .is_some_and(|value| !value.trim().is_empty())
            };
            filled("verifiedOn") && filled("verifiedBy")
        })
        .map(|(identifier, _)| identifier.clone())
        .collect()
}

fn is_permitted(identifier: &str, allowlist: &[String]) -> bool {
    identifier.starts_with(OWN_NAMESPACE) || allowlist.iter().any(|a| a == identifier)
}

/// Collect every `semanticId` in a serialised AAS document, with its path.
///
/// Walks the JSON rather than the Rust types deliberately: it reaches every
/// nesting depth, survives the element enum gaining a variant, and sees exactly
/// what a consumer parsing our output would see.
fn collect_semantic_ids(node: &serde_json::Value, path: &str, found: &mut Vec<(String, String)>) {
    match node {
        serde_json::Value::Object(map) => {
            for (key, value) in map {
                let child = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                if key == "semanticId" {
                    for entry in value["keys"].as_array().into_iter().flatten() {
                        if let Some(id) = entry["value"].as_str() {
                            found.push((child.clone(), id.to_owned()));
                        }
                    }
                }
                collect_semantic_ids(value, &child, found);
            }
        }
        serde_json::Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                collect_semantic_ids(item, &format!("{path}[{i}]"), found);
            }
        }
        _ => {}
    }
}

/// Nothing we emit may claim a third party's vocabulary without provenance.
#[test]
fn every_emitted_semantic_id_is_ours_or_provenanced() {
    let allowlist = allowlisted_identifiers();
    let mut checked = 0usize;

    for (sector, data, version, _) in all_sector_cases() {
        let key = sector.catalog_key().to_owned();
        let passport = base(sector, data, version);
        let (shell, submodels) =
            build_aas_from_passport(&passport, VALID_GTIN, Audience::Public).expect("masking");

        let document = serde_json::json!({
            "shell": serde_json::to_value(&shell).unwrap(),
            "submodels": serde_json::to_value(&submodels).unwrap(),
        });
        let mut found = Vec::new();
        collect_semantic_ids(&document, "", &mut found);

        assert!(
            !found.is_empty(),
            "sector '{key}' emitted no semanticIds at all — the walker is broken, \
             not the output"
        );

        for (path, id) in &found {
            assert!(
                is_permitted(id, &allowlist),
                "sector '{key}' emits unprovenanced semanticId '{id}' at '{path}'. \
                 Either move it into the urn:odal-node: namespace, or add a \
                 verified entry to semantic_ids/allowlist.json."
            );
        }
        checked += found.len();
    }

    // The generic escape hatch is walked too. It is a separate code path with
    // its own hardcoded semanticId, and a gate that only walked `sectors/` is
    // precisely how a coined `urn:idta:` identifier survived there unnoticed.
    let generic = map_dpp_to_aas_submodel(
        "urn:odal-node:dpp:test:generic",
        &serde_json::json!({ "productName": "Widget", "massKg": 2.5 }),
    );
    let mut found = Vec::new();
    collect_semantic_ids(&serde_json::to_value(&generic).unwrap(), "", &mut found);
    assert!(
        !found.is_empty(),
        "the generic mapper emitted no semanticIds — the walker missed its path"
    );
    for (path, id) in &found {
        assert!(
            is_permitted(id, &allowlist),
            "the generic mapper emits unprovenanced semanticId '{id}' at '{path}'"
        );
    }
    checked += found.len();

    assert!(checked > 0, "the gate asserted nothing");
}

/// Every sector's submodel template identifier, including the ones no passport
/// fixture in this file exercises.
///
/// Battery is covered elsewhere for its mapping, so the walk above never builds
/// one. Its template identifier would otherwise go unchecked entirely.
#[test]
fn every_sector_template_semantic_id_is_ours_or_provenanced() {
    let allowlist = allowlisted_identifiers();
    let catalog = dpp_domain::catalog::SectorCatalog::new();

    for descriptor in catalog.all().iter() {
        let key = descriptor.key.as_str();
        let Some(template) = sector_submodel_template(key) else {
            continue;
        };
        assert!(
            is_permitted(template.semantic_id, &allowlist),
            "sector template '{key}' carries unprovenanced semanticId \
             '{}'",
            template.semantic_id
        );
    }
}

/// A record without a reader is not provenance.
#[test]
fn an_allowlist_entry_missing_provenance_is_refused() {
    let complete = serde_json::json!({
        "allowlist": {
            "urn:example:concept": { "verifiedOn": "2026-01-01", "verifiedBy": "A. Reader" }
        }
    });
    assert_eq!(
        allowlisted_from(&complete),
        vec!["urn:example:concept".to_owned()],
        "a complete record should be honoured"
    );

    for (missing, doc) in [
        (
            "no reader",
            serde_json::json!({"allowlist": {"urn:example:concept": {"verifiedOn": "2026-01-01"}}}),
        ),
        (
            "no date",
            serde_json::json!({"allowlist": {"urn:example:concept": {"verifiedBy": "A. Reader"}}}),
        ),
        (
            "blank reader",
            serde_json::json!({"allowlist": {"urn:example:concept": {"verifiedOn": "2026-01-01", "verifiedBy": "   "}}}),
        ),
    ] {
        assert!(
            allowlisted_from(&doc).is_empty(),
            "an entry with {missing} was honoured — a half-filled record must fail \
             the gate exactly like an absent one"
        );
    }
}

/// The live allowlist is empty, and that is a decision.
///
/// This crate emits no third-party semanticIds. The test exists so that adding
/// one is a deliberate act with a visible diff here, rather than something that
/// slips in — and so an empty allowlist is never mistaken for a broken loader.
#[test]
fn no_third_party_identifier_is_currently_permitted() {
    assert!(
        allowlisted_identifiers().is_empty(),
        "a third-party identifier was allowlisted; confirm a named reader checked \
         it against the authority's own source, then update this test"
    );
}

/// A plausible-looking identifier is exactly the thing this gate exists to
/// catch — it must not pass on the strength of looking official.
#[test]
fn a_fabricated_third_party_identifier_is_refused() {
    let allowlist = allowlisted_identifiers();
    for fake in [
        "urn:eclass:0173-1#01-XXXXXX#001",
        "urn:idta:aas:submodel:digital-product-passport:1.0",
        "https://admin-shell.io/IDTA/02023/0/9",
    ] {
        assert!(
            !is_permitted(fake, &allowlist),
            "'{fake}' passed the gate — a coined identifier in a standards-body \
             namespace is the defect this test exists for"
        );
    }
}

/// Both sections carry a fixed key set.
///
/// Entries accreted per-entry keys once already — some carrying `finding`,
/// others `whyWithdrawn`, others neither — which makes the file unreadable as
/// data and lets a required field go missing without anything noticing. The
/// shape is documented in the file's own `$comment` and enforced here.
#[test]
fn every_entry_carries_its_section_key_set() {
    const ALLOWLIST_KEYS: &[&str] = &[
        "authority",
        "source",
        "release",
        "meaning",
        "usedFor",
        "licence",
        "verifiedOn",
        "verifiedBy",
    ];
    const TRACKED_KEYS: &[&str] = &[
        "authority",
        "source",
        "usedFor",
        "status",
        "checkedOn",
        "finding",
        "correctIdentifier",
        "licence",
        "nextStep",
    ];

    let doc = allowlist_document();
    for (section, required) in [("allowlist", ALLOWLIST_KEYS), ("tracked", TRACKED_KEYS)] {
        let entries = doc[section]
            .as_object()
            .unwrap_or_else(|| panic!("`{section}` is an object"));

        for (identifier, record) in entries {
            let record = record
                .as_object()
                .unwrap_or_else(|| panic!("{section}['{identifier}'] is an object"));

            let mut actual: Vec<&str> = record.keys().map(String::as_str).collect();
            actual.sort_unstable();
            let mut expected = required.to_vec();
            expected.sort_unstable();

            assert_eq!(
                actual, expected,
                "{section}['{identifier}'] has the wrong key set — every entry                  carries every key of its section, with null for anything not                  established"
            );
        }
    }
}

/// Every identifier in the research record stays refused.
///
/// `tracked` documents identifiers we investigated and did not adopt, with the
/// correct value where it is known. It is a note to a future reader, and this
/// test is what stops it becoming a second, softer allowlist: promoting one
/// means moving it into `allowlist` **and** naming who read the source, not
/// editing a status string.
#[test]
fn nothing_in_the_research_record_is_permitted() {
    let allowlist = allowlisted_identifiers();
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../dpp-aas/src/semantic_ids/allowlist.json"
    );
    let raw = std::fs::read_to_string(path).expect("the allowlist file is present");
    let doc: serde_json::Value = serde_json::from_str(&raw).expect("valid JSON");

    let tracked = doc["tracked"]
        .as_object()
        .expect("the file carries a `tracked` record");
    assert!(
        !tracked.is_empty(),
        "the research record should not be empty"
    );

    for identifier in tracked.keys() {
        assert!(
            !is_permitted(identifier, &allowlist),
            "'{identifier}' is in `tracked` but the gate permits it — a tracked \
             identifier must never also be allowlisted"
        );
    }
}

/// The one sector permitted a typed mapper while still `provisional` in the
/// catalog: it carries the active pilot's branching code, and deleting that to
/// satisfy a rule would be the rule stretching the work rather than governing
/// it. Recorded here, deliberately, rather than assumed — one sector at a time.
const PILOT_CARVE_OUT: &str = "textile";

/// A typed mapper exists **only** for a sector whose act is in force, plus the
/// recorded pilot carve-out. Everything else renders through the generic
/// key-value projection.
///
/// This is the inverse of what this file used to assert. The old shape required
/// a dedicated submodel for *every* catalog sector, which meant a newly
/// announced sector could not be added as catalog data alone — it needed Rust
/// code for an act that binds nobody yet, and a hand-written AAS template for a
/// submodel template that does not exist. A generic projection is the honest
/// rendering of a sector whose ratified template has not been published.
///
/// Driven by the catalog rather than a hardcoded list, so a sector coming into
/// force flips this expectation by changing its manifest — and this test then
/// demands the typed mapper that its new status has earned.
#[test]
fn only_in_force_sectors_carry_a_typed_mapper() {
    let catalog = dpp_domain::SectorCatalog::new();

    for (sector, data, version, _id_short) in all_sector_cases() {
        let key = sector.catalog_key().to_owned();
        let passport = base(sector, data, version);
        let (_, submodels) =
            build_aas_from_passport(&passport, VALID_GTIN, Audience::Public).expect("masking");

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
            .expect("a sector submodel is always present");

        // The generic projection is the one named for the field it renders;
        // a typed mapper names its own submodel template.
        let is_typed = sector_submodel.id_short != "SectorData";
        let may_be_typed = catalog.is_in_force(&key) || key == PILOT_CARVE_OUT;

        assert_eq!(
            is_typed, may_be_typed,
            "sector '{key}': typed mapper present = {is_typed}, but permitted = {may_be_typed}. \
             A provisional sector must render generically; an in-force one must not."
        );
    }
}

// ─── metamodel validity gate ─────────────────────────────────────────────────
//
// Everything else in this file asserts our output against our own expectations.
// This section asserts it against IDTA's, using their published schemas as an
// external oracle — the only check here that can tell us the document is
// ingestible by somebody else's AAS toolchain rather than merely self-consistent.
//
// **Every revision, not one.** An Environment is validated against 3.0, 3.1 and
// 3.2 together, because no single revision is the strictest and picking one
// means choosing which half of a rule to stop enforcing. The `idShort` rule is
// the case in point, and it is the rule that matters most here — the generic
// mapper builds `idShort`s from operator-supplied JSON keys, so it is the one
// place where a passport's *contents* decide whether our output is legal:
//
//   idShort             3.0      3.1/3.2
//   state-of-health     reject   accept    (3.1 permits interior hyphens)
//   a                   accept   reject    (3.1 requires two or more chars)
//
// Validating against the intersection is what "an integrator's toolchain,
// whichever revision it implements" actually means.
//
// Schema-valid is not IDTA-conformant. See `fixtures/aas/NOTICE.md`.

/// The vendored revisions, with the number of UTF-16-only `pattern` constraints
/// each one carries (see [`strip_utf16_only_patterns`]).
///
/// Counts are pinned per revision so a schema update that changes one fails here
/// and gets looked at, rather than quietly dropping more constraints than
/// intended.
const VENDORED_SCHEMAS: &[(&str, &str, usize)] = &[
    (
        "3.0",
        concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/aas/aas-3.0.json"),
        28,
    ),
    (
        "3.1",
        concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/aas/aas-3.1.json"),
        33,
    ),
    (
        "3.2",
        concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/aas/aas-3.2.json"),
        38,
    ),
];

/// Strip the one `pattern` a Rust regex engine cannot compile, returning how
/// many were removed.
///
/// IDTA writes the "valid XML character" rule with UTF-16 surrogate-pair
/// alternations (`\ud800[\udc00-\udfff]`, …). That is well-formed for a
/// JavaScript regex engine, which matches over UTF-16 code units; Rust's
/// `regex` matches over Unicode scalar values, where a lone surrogate is not a
/// character at all, so the pattern is rejected outright and the whole schema
/// fails to compile.
///
/// Dropping it costs nothing we care about. It constrains strings to exclude
/// control characters and unpaired surrogates — neither of which Rust's `String`
/// can represent in the unpaired case, nor our mappers emit in the other. Every
/// constraint that carries meaning for this crate survives, including the
/// `idShort` name rule, which is the one that actually governs whether our
/// generated names are legal — and which differs between revisions, so it is
/// enforced against all of them rather than one.
fn strip_utf16_only_patterns(node: &mut serde_json::Value) -> usize {
    match node {
        serde_json::Value::Object(map) => {
            let hit = map
                .get("pattern")
                .and_then(|p| p.as_str())
                .is_some_and(|p| p.contains("\\ud800"));
            let mut removed = usize::from(hit);
            if hit {
                map.remove("pattern");
            }
            for value in map.values_mut() {
                removed += strip_utf16_only_patterns(value);
            }
            removed
        }
        serde_json::Value::Array(items) => items.iter_mut().map(strip_utf16_only_patterns).sum(),
        _ => 0,
    }
}

/// Every vendored revision, compiled once, in the order declared above.
fn aas_validators() -> &'static [(&'static str, jsonschema::Validator)] {
    static VALIDATORS: std::sync::OnceLock<Vec<(&'static str, jsonschema::Validator)>> =
        std::sync::OnceLock::new();
    VALIDATORS.get_or_init(|| {
        VENDORED_SCHEMAS
            .iter()
            .map(|(revision, path, expected_patterns)| {
                let raw = std::fs::read_to_string(path).unwrap_or_else(|e| {
                    panic!("the vendored AAS {revision} schema is present: {e}")
                });
                let mut schema: serde_json::Value =
                    serde_json::from_str(&raw).expect("the schema is JSON");

                let removed = strip_utf16_only_patterns(&mut schema);
                assert_eq!(
                    removed, *expected_patterns,
                    "AAS {revision}: expected to drop exactly {expected_patterns} UTF-16-only \
                     `pattern` constraints, dropped {removed}. The vendored schema changed: \
                     re-check what is being removed before updating this count — see \
                     fixtures/aas/NOTICE.md"
                );

                let validator = jsonschema::validator_for(&schema)
                    .unwrap_or_else(|e| panic!("the vendored AAS {revision} schema compiles: {e}"));
                (*revision, validator)
            })
            .collect()
    })
}

/// Validate `document` against **every** vendored revision, reporting each
/// violation with the revision that raised it and its instance path.
///
/// All revisions rather than the first failure: a document rejected by 3.2 and
/// accepted by 3.0 is exactly the case worth seeing in full, and reporting one
/// error at a time would mean one fix per run when a wrong mapper is usually
/// wrong in several places at once.
fn assert_valid_aas(document: &serde_json::Value, what: &str) {
    let errors: Vec<String> = aas_validators()
        .iter()
        .flat_map(|(revision, validator)| {
            validator
                .iter_errors(document)
                .map(move |e| format!("  [{revision}] at `{}`: {e}", e.instance_path()))
        })
        .collect();
    assert!(
        errors.is_empty(),
        "{what} is not valid against every vendored AAS revision — {} violation(s):\n{}",
        errors.len(),
        errors.join("\n")
    );
}

/// Whether **any** vendored revision rejects `document`.
fn rejected_by_any(document: &serde_json::Value) -> bool {
    aas_validators()
        .iter()
        .any(|(_, v)| v.iter_errors(document).next().is_some())
}

/// The revisions that reject `document`, for assertions about where the
/// revisions disagree.
fn rejecting_revisions(document: &serde_json::Value) -> Vec<&'static str> {
    aas_validators()
        .iter()
        .filter(|(_, v)| v.iter_errors(document).next().is_some())
        .map(|(revision, _)| *revision)
        .collect()
}

/// Guard: the validator must actually reject something.
///
/// A schema that failed to compile its constraints, or a validator wired up to
/// accept anything, would make every assertion below pass while proving
/// nothing. `Environment`'s own members are all optional, so an empty document
/// is legitimately valid and proves nothing either way; this uses a document
/// that is positively wrong instead — a shell missing its required
/// `assetInformation`.
#[test]
fn the_aas_validator_rejects_an_invalid_document() {
    let bogus = serde_json::json!({
        "assetAdministrationShells": [
            { "id": "urn:x", "modelType": "AssetAdministrationShell" }
        ]
    });
    // Asserted per revision, not "some revision rejects it": one compiled
    // validator silently accepting everything would still leave the others
    // failing the document, and the gate would look healthy.
    for (revision, validator) in aas_validators() {
        assert!(
            validator.iter_errors(&bogus).next().is_some(),
            "AAS {revision} accepted a shell with no assetInformation — that \
             validator is not enforcing its schema, so every validity assertion \
             resting on it is vacuous"
        );
    }
    assert_eq!(
        aas_validators().len(),
        VENDORED_SCHEMAS.len(),
        "a vendored revision failed to compile and was dropped silently"
    );
}

/// The shapes this crate used to emit are rejected, one by one.
///
/// Each of these produced a document no AAS parser would accept, and each was
/// invisible for as long as nothing validated the output. Asserting the
/// *rejections* — not just that today's output passes — is what makes the four
/// fixes durable: a refactor that reintroduces any one of them fails here with
/// the reason named, rather than somewhere downstream in a partner's toolchain.
#[test]
fn the_previously_emitted_invalid_shapes_are_rejected() {
    let valid_shell = |extra: serde_json::Value| {
        let mut shell = serde_json::json!({
            "id": "urn:odal-node:aas:x",
            "idShort": "DigitalProductPassport",
            "modelType": "AssetAdministrationShell",
            "assetInformation": {
                "assetKind": "Instance",
                "globalAssetId": "urn:odal-node:product:09506000134352"
            }
        });
        let (obj, extra) = (shell.as_object_mut().unwrap(), extra);
        for (k, v) in extra.as_object().unwrap() {
            obj.insert(k.clone(), v.clone());
        }
        serde_json::json!({ "assetAdministrationShells": [shell] })
    };

    for (what, document) in [
        (
            "AssetInformation without the required assetKind",
            serde_json::json!({"assetAdministrationShells": [{
                "id": "urn:odal-node:aas:x",
                "modelType": "AssetAdministrationShell",
                "assetInformation": { "globalAssetId": "urn:odal-node:product:09506000134352" }
            }]}),
        ),
        (
            "a shell whose submodels are bare {id} objects rather than References",
            valid_shell(
                serde_json::json!({ "submodels": [{ "id": "urn:odal-node:dpp:x:manufacturer-information" }] }),
            ),
        ),
        (
            "a submodel element typed modelType: Reference",
            serde_json::json!({"submodels": [{
                "id": "urn:odal-node:dpp:x:manufacturer-information",
                "modelType": "Submodel",
                "submodelElements": [{
                    "modelType": "Reference",
                    "idShort": "didWebUrl",
                    "value": "https://example.com/.well-known/did.json"
                }]
            }]}),
        ),
        (
            "a ReferenceElement whose value is a bare string",
            serde_json::json!({"submodels": [{
                "id": "urn:odal-node:dpp:x:manufacturer-information",
                "modelType": "Submodel",
                "submodelElements": [{
                    "modelType": "ReferenceElement",
                    "idShort": "didWebUrl",
                    "value": "https://example.com/.well-known/did.json"
                }]
            }]}),
        ),
        (
            "an empty conceptDescriptions array",
            serde_json::json!({ "conceptDescriptions": [] }),
        ),
        (
            "a submodel with an empty submodelElements array",
            serde_json::json!({"submodels": [{
                "id": "urn:odal-node:dpp:x:repairability",
                "modelType": "Submodel",
                "submodelElements": []
            }]}),
        ),
    ] {
        // Every revision, not merely one: these are metamodel defects rather
        // than revision-specific rules, so a revision that started accepting one
        // would mean the vendored bytes are not what we think they are.
        let accepting: Vec<&str> = aas_validators()
            .iter()
            .filter(|(_, v)| v.iter_errors(&document).next().is_none())
            .map(|(revision, _)| *revision)
            .collect();
        assert!(
            accepting.is_empty(),
            "AAS {accepting:?} accepted {what} — this gate no longer protects against it"
        );
    }
}

/// The `idShort` name rule differs between revisions, in **both** directions.
///
/// This is the whole reason every Environment is validated against all three
/// rather than a chosen one. `idShort` is also the rule that matters most here:
/// the generic mapper builds names from operator-supplied JSON keys, so it is
/// the one place where a passport's *contents*, not our code, decide whether the
/// output is legal.
///
///   3.0      `^[a-zA-Z][a-zA-Z0-9_]*$`
///   3.1/3.2  `^[a-zA-Z][a-zA-Z0-9_-]*[a-zA-Z0-9_]+$`
///
/// So 3.1 permits interior hyphens that 3.0 forbids, and requires two or more
/// characters where 3.0 accepts one. Neither is the stricter revision, and
/// validating against either alone leaves the other's rule unenforced.
///
/// Asserted rather than described, because this is the claim `NOTICE.md` rests
/// on: if a future revision converges the two rules, this test says so.
#[test]
fn the_idshort_rule_diverges_between_revisions() {
    let submodel_named = |name: &str| {
        serde_json::json!({"submodels": [{
            "id": "urn:odal-node:dpp:x:sector-data",
            "idShort": name,
            "modelType": "Submodel"
        }]})
    };

    assert_eq!(
        rejecting_revisions(&submodel_named("state-of-health-pct")),
        vec!["3.0"],
        "only 3.0 should reject an interior hyphen"
    );
    assert_eq!(
        rejecting_revisions(&submodel_named("a")),
        vec!["3.1", "3.2"],
        "only 3.1 and 3.2 should reject a single-character name"
    );

    // Both ends stay illegal everywhere, so the divergence is genuinely about
    // the middle of the rule rather than the rule having been abandoned.
    for always_illegal in ["trailing-", "-leading", "9numeric"] {
        assert_eq!(
            rejecting_revisions(&submodel_named(always_illegal)).len(),
            VENDORED_SCHEMAS.len(),
            "'{always_illegal}' must be rejected by every revision"
        );
    }

    // And a name we actually emit is legal under all of them.
    assert!(
        !rejected_by_any(&submodel_named("StateOfHealthPct")),
        "the naming style this crate emits must satisfy every revision"
    );
}

// ─── the defect the schema cannot see ────────────────────────────────────────
//
// IDTA sets `additionalProperties` nowhere in the whole schema, so a member that
// is not part of a class validates in silence. Everything above would pass with
// arbitrary extra members on every object. A strict AAS loader would not, and it
// is the loader that decides whether a partner can open our output.

/// The vendored schema documents, unmodified — the validators above strip a
/// pattern from their own copies, which is the wrong thing to read class members
/// from.
fn aas_schema_documents() -> &'static [(&'static str, serde_json::Value)] {
    static DOCUMENTS: std::sync::OnceLock<Vec<(&'static str, serde_json::Value)>> =
        std::sync::OnceLock::new();
    DOCUMENTS.get_or_init(|| {
        VENDORED_SCHEMAS
            .iter()
            .map(|(revision, path, _)| {
                let raw = std::fs::read_to_string(path).unwrap_or_else(|e| {
                    panic!("the vendored AAS {revision} schema is present: {e}")
                });
                (
                    *revision,
                    serde_json::from_str(&raw).expect("the schema is JSON"),
                )
            })
            .collect()
    })
}

/// Every member name a metamodel class defines, following `allOf` and `$ref`.
///
/// Derived from the schema rather than restated here. A hand-written member list
/// would be a second copy of the metamodel in this repo, and the argument
/// against that is the one this crate already makes for `is_placeholder`: a
/// value that can be derived from another must be, or the two will eventually
/// disagree and the wrong one will be the one somebody trusts.
///
/// Descends into `allOf` branches and `$ref` targets — the inheritance chain —
/// but never into a property's own value, which would pull in the members of
/// whatever class that property happens to be typed as.
///
/// Read from one revision. Use [`members_common_to_every_revision`] for the
/// gate itself.
fn members_of_in(document: &serde_json::Value, class: &str) -> std::collections::BTreeSet<String> {
    fn walk(
        document: &serde_json::Value,
        node: &serde_json::Value,
        out: &mut std::collections::BTreeSet<String>,
        depth: usize,
    ) {
        assert!(depth < 16, "the schema's $ref chain does not terminate");
        if let Some(reference) = node.get("$ref").and_then(serde_json::Value::as_str) {
            let name = reference
                .rsplit('/')
                .next()
                .expect("a $ref names a definition");
            if let Some(target) = document["definitions"].get(name) {
                walk(document, target, out, depth + 1);
            }
            return;
        }
        if let Some(properties) = node
            .get("properties")
            .and_then(serde_json::Value::as_object)
        {
            out.extend(properties.keys().cloned());
        }
        for branch in node
            .get("allOf")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
        {
            walk(document, branch, out, depth + 1);
        }
    }

    let mut out = std::collections::BTreeSet::new();
    walk(document, &document["definitions"][class], &mut out, 0);
    out
}

/// The members every vendored revision recognises for `class` — their
/// intersection.
///
/// The intersection rather than any one revision's set, for the same reason the
/// validators run as a set: a member only some revisions know is a member that
/// some toolchain will refuse. If the revisions ever disagree about a class we
/// emit, the gate should tighten to what they share, not pick a winner.
fn members_common_to_every_revision(class: &str) -> std::collections::BTreeSet<String> {
    let mut revisions = aas_schema_documents()
        .iter()
        .map(|(_, document)| members_of_in(document, class));
    let first = revisions.next().expect("at least one vendored revision");
    revisions.fold(first, |acc, next| {
        acc.intersection(&next).cloned().collect()
    })
}

/// The shell carries no member the metamodel does not define.
///
/// We emitted `kind` on `AssetAdministrationShell` for several releases. It is
/// not a member of that class — `kind` comes from `HasKind`, which `Submodel`
/// composes and the shell does not — and every gate above accepted it, because
/// a JSON Schema without `additionalProperties` has no opinion about members it
/// has never heard of.
///
/// Written over the whole member set rather than as "`kind` is absent", because
/// the next one will not be called `kind`.
#[test]
fn the_shell_carries_no_member_outside_the_metamodel() {
    let allowed = members_common_to_every_revision("AssetAdministrationShell");

    // 3.0, 3.1 and 3.2 currently define this class identically. Recorded as an
    // assertion rather than assumed, because the intersection above would
    // silently narrow if a future revision dropped a member, and a gate that
    // quietly tightens is one that fails for a reason nobody can read.
    for (revision, document) in aas_schema_documents() {
        assert_eq!(
            members_of_in(document, "AssetAdministrationShell"),
            allowed,
            "AAS {revision} defines AssetAdministrationShell differently from the \
             other vendored revisions. The gate below has narrowed to what they \
             share — confirm that is what you want before updating this assertion"
        );
    }

    // Guards on the derivation itself: one that returned everything, or nothing,
    // would make the loop below pass while proving the opposite of what it says.
    for expected in [
        "id",
        "idShort",
        "modelType",
        "assetInformation",
        "submodels",
    ] {
        assert!(
            allowed.contains(expected),
            "the member derivation missed '{expected}' — it is not reading the \
             inheritance chain, so this gate permits too little"
        );
    }
    assert!(
        !allowed.contains("kind"),
        "the derivation admitted 'kind', which reaches the shell only through \
         `HasKind` — a class it does not compose. It is following something it \
         should not, so this gate permits too much"
    );

    let mut checked = 0usize;
    for (sector, data, version, _) in all_sector_cases() {
        let key = sector.catalog_key().to_owned();
        let passport = base(sector, data, version);
        let environment = build_aas_environment(&passport, VALID_GTIN, Audience::Public)
            .expect("a public projection is buildable");
        let document = serde_json::to_value(&environment).expect("serialises");

        for shell in document["assetAdministrationShells"]
            .as_array()
            .into_iter()
            .flatten()
        {
            for member in shell.as_object().expect("a shell is an object").keys() {
                assert!(
                    allowed.contains(member.as_str()),
                    "sector '{key}': the shell carries '{member}', which \
                     `AssetAdministrationShell` does not define. The schema gate will \
                     never catch this — IDTA sets `additionalProperties` nowhere — but \
                     a strict AAS loader rejects the document."
                );
            }
            checked += 1;
        }
    }
    assert!(checked > 0, "the gate asserted nothing");
}

/// Every sector's public Environment validates against IDTA's own schema.
#[test]
fn every_sector_environment_is_schema_valid() {
    let mut checked = 0usize;
    for (sector, data, version, _) in all_sector_cases() {
        let key = sector.catalog_key().to_owned();
        let passport = base(sector, data, version);
        let environment = build_aas_environment(&passport, VALID_GTIN, Audience::Public)
            .expect("a public projection is buildable");
        let document = serde_json::to_value(&environment).expect("serialises");
        assert_valid_aas(
            &document,
            &format!("the public Environment for sector '{key}'"),
        );
        checked += 1;
    }
    assert!(checked > 0, "the gate asserted nothing");
}

/// The generic fallback path is validated too.
///
/// It builds `idShort`s from arbitrary operator-supplied JSON keys, and
/// `idShort` carries a `^[a-zA-Z][a-zA-Z0-9_]*$` pattern — so it is the one
/// path where a passport's *contents*, not our code, decide whether the output
/// is valid. Covered here so that stays true by test rather than by luck.
#[test]
fn the_generic_sector_environment_is_schema_valid() {
    let other = SectorData::other(serde_json::json!({
        "sector": "spacecraft",
        "thrustKn": 500.0,
        "reusable": true,
        "stageCount": 2
    }))
    .expect("spacecraft has no typed variant");
    let passport = base(Sector::Other("spacecraft".into()), other, "1.0.0");
    let environment =
        build_aas_environment(&passport, VALID_GTIN, Audience::Public).expect("buildable");
    assert_valid_aas(
        &serde_json::to_value(&environment).expect("serialises"),
        "the generic-fallback Environment",
    );
}

/// A passport stripped to almost nothing still produces a valid document.
///
/// This is the case the `minItems: 1` constraints punish: submodels whose every
/// field is absent serialise to an empty `submodelElements`, and an empty array
/// is invalid where an absent one is fine. Masking produces exactly this shape
/// whenever a sector's public tier is thin, so it is not a hypothetical.
#[test]
fn a_sparse_passport_environment_is_schema_valid() {
    let mut passport = base(
        Sector::Electronics,
        SectorData::Electronics(electronics_data()),
        "1.0.0",
    );
    passport.sector_data = None;
    passport.materials = Vec::new();
    passport.co2e_per_unit = None;
    passport.repairability_score = None;

    let environment =
        build_aas_environment(&passport, VALID_GTIN, Audience::Public).expect("buildable");
    let document = serde_json::to_value(&environment).expect("serialises");
    assert_valid_aas(&document, "a sparse passport's Environment");

    // And specifically: no empty array reached the wire anywhere in it.
    fn no_empty_arrays(node: &serde_json::Value, path: &str) {
        match node {
            serde_json::Value::Array(items) => {
                assert!(!items.is_empty(), "empty array emitted at `{path}`");
                for (i, item) in items.iter().enumerate() {
                    no_empty_arrays(item, &format!("{path}[{i}]"));
                }
            }
            serde_json::Value::Object(map) => {
                for (k, v) in map {
                    no_empty_arrays(v, &format!("{path}.{k}"));
                }
            }
            _ => {}
        }
    }
    no_empty_arrays(&document, "");
}

// ─── committed Environments ──────────────────────────────────────────────────
//
// One AAS Environment per product group, checked in under
// `fixtures/aas/environments/`. They do three jobs a passing assertion cannot:
//
//   - a mapper change arrives as a reviewable JSON diff rather than a green run,
//   - they are an artefact a partner can open in their own AAS tooling, and
//   - they let a human read what we actually emit, which is how the four
//     metamodel defects went unnoticed for several releases.
//
// Regenerate with `UPDATE_AAS_FIXTURES=1 cargo test -p dpp-tests`, and **read
// the diff** — that is the point of the exercise, not a step to get past.

/// A passport with every clock- and randomness-derived field pinned.
///
/// `base_passport` uses `PassportId::new()` and `Utc::now()`, so its output
/// differs on every call. That is right for the assertions elsewhere in this
/// file and fatal for a committed fixture, which must be byte-stable or its
/// diff is noise.
fn pinned(sector: Sector, data: SectorData, version: &str) -> dpp_domain::Passport {
    let fixed = "2026-01-01T00:00:00Z"
        .parse::<DateTime<Utc>>()
        .expect("a date");
    let mut passport = base(sector, data, version);
    passport.id = dpp_domain::PassportId(uuid::uuid!("01234567-89ab-7cde-8f01-23456789abcd"));
    passport.created_at = fixed;
    passport.updated_at = fixed;
    passport
}

/// Battery, built from its wire form.
///
/// It is absent from `all_sector_cases` (its mapping is exercised by the
/// end-to-end battery test), but it is the reference product group and carries
/// more non-public fields than any other, so a committed Environment that
/// omitted it would omit the one worth reading. Restricted and individual-tier
/// fields are present deliberately: their absence from the committed output is
/// what a reviewer should be checking.
fn battery_case() -> (Sector, SectorData, &'static str) {
    let data = serde_json::from_value(serde_json::json!({
        "sector": "battery",
        "gtin": VALID_GTIN,
        "batteryChemistry": "LFP",
        "nominalVoltageV": 3.7,
        "nominalCapacityAh": 50.0,
        "expectedLifetimeCycles": 1000,
        "co2ePerUnitKg": 85.0,
        "anodeMaterial": [{ "name": "graphite", "weightPct": 45.0 }],
        "cathodeMaterial": [{ "name": "lithium-iron-phosphate", "weightPct": 30.0 }],
        "electrolyteMaterial": [{ "name": "LiPF6", "weightPct": 12.0 }],
        "dueDiligenceUrl": "https://acme.example.com/due-diligence",
        "disassemblyInstructionsUrl": "https://acme.example.com/disassembly",
        "sohMethodology": "IEC 62660-1 capacity fade",
        "stateOfHealthPct": 97.5
    }))
    .expect("the battery wire form is valid");
    (Sector::Battery, data, "1.0.0")
}

/// Collect every `idShort` in a serialised AAS document, at any depth.
fn walk_id_shorts(node: &serde_json::Value, out: &mut std::collections::BTreeSet<String>) {
    match node {
        serde_json::Value::Object(map) => {
            if let Some(name) = map.get("idShort").and_then(|v| v.as_str()) {
                out.insert(name.to_owned());
            }
            map.values().for_each(|child| walk_id_shorts(child, out));
        }
        serde_json::Value::Array(items) => {
            items.iter().for_each(|item| walk_id_shorts(item, out));
        }
        _ => {}
    }
}

fn environment_fixture_path(sector_key: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/aas/environments")
        .join(format!("{sector_key}.json"))
}

/// Every product group's public Environment matches its committed fixture.
#[test]
fn committed_environments_match_what_the_mappers_produce() {
    let updating = std::env::var_os("UPDATE_AAS_FIXTURES").is_some();
    let mut cases: Vec<(Sector, SectorData, &str)> = all_sector_cases()
        .into_iter()
        .map(|(s, d, v, _)| (s, d, v))
        .collect();
    cases.push(battery_case());

    let mut stale = Vec::new();
    for (sector, data, version) in cases {
        let key = sector.catalog_key().to_owned();
        let passport = pinned(sector, data, version);
        let environment = build_aas_environment(&passport, VALID_GTIN, Audience::Public)
            .expect("a public projection is buildable");
        // Pretty-printed, with a trailing newline: these are read by people and
        // diffed by git, not parsed by us.
        let rendered = format!(
            "{}\n",
            serde_json::to_string_pretty(&environment).expect("serialises")
        );

        let path = environment_fixture_path(&key);
        if updating {
            std::fs::create_dir_all(path.parent().expect("has a parent"))
                .expect("fixture directory is writable");
            std::fs::write(&path, &rendered).expect("fixture is writable");
            continue;
        }

        match std::fs::read_to_string(&path) {
            Ok(committed) if committed.replace("\r\n", "\n") == rendered => {}
            Ok(_) => stale.push(key),
            Err(_) => stale.push(format!("{key} (missing)")),
        }
    }

    assert!(
        updating || stale.is_empty(),
        "the committed Environments for {stale:?} no longer match what the mappers \
         produce. Regenerate with `UPDATE_AAS_FIXTURES=1 cargo test -p dpp-tests` \
         and read the diff before committing it — an unexplained change to this \
         output is a change to what every AAS consumer receives."
    );
}

/// The committed fixtures are themselves valid AAS, and carry no non-public field.
///
/// Without this they are only a record of what we emit. With it they are a
/// second, independent check on the same properties, run against bytes a human
/// can read — so a reviewer looking at the diff is looking at something the
/// gate has also inspected.
#[test]
fn committed_environments_are_valid_and_carry_nothing_non_public() {
    let catalog = dpp_domain::SectorCatalog::new();
    let mut checked = 0usize;

    for descriptor in catalog.all().iter() {
        let key = descriptor.key.as_str();
        let path = environment_fixture_path(key);
        let Ok(raw) = std::fs::read_to_string(&path) else {
            continue;
        };
        let document: serde_json::Value = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("committed Environment for '{key}' is not JSON: {e}"));

        assert_valid_aas(&document, &format!("the committed Environment for '{key}'"));

        // Matched as whole `idShort`s, not as substrings of the rendered text.
        // A substring test reports `expectedLifetime` (individual) inside
        // `expectedLifetimeCycles` (public) — it fails on a field that is
        // supposed to be there, which is how a gate teaches people to ignore it.
        let mut emitted = std::collections::BTreeSet::new();
        walk_id_shorts(&document, &mut emitted);

        for (field, class) in &descriptor.disclosure {
            if *class == dpp_domain::Disclosure::Public {
                continue;
            }
            assert!(
                !emitted.contains(field),
                "the committed Environment for '{key}' carries '{field}', which \
                 that sector classifies as {class:?}"
            );
        }
        checked += 1;
    }

    assert!(
        checked >= 11,
        "only {checked} committed Environments were found; the fixtures are \
         missing. Generate them with `UPDATE_AAS_FIXTURES=1 cargo test -p dpp-tests`"
    );
}

// ─── disclosure masking gate ─────────────────────────────────────────────────

/// Every `idShort` in the projection, at any nesting depth.
fn emitted_id_shorts(submodels: &[dpp_aas::AasSubmodel]) -> Vec<String> {
    fn walk(value: &serde_json::Value, out: &mut Vec<String>) {
        match value {
            serde_json::Value::Object(map) => {
                if let Some(s) = map.get("idShort").and_then(|v| v.as_str()) {
                    out.push(s.to_owned());
                }
                map.values().for_each(|v| walk(v, out));
            }
            serde_json::Value::Array(items) => items.iter().for_each(|v| walk(v, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    walk(
        &serde_json::to_value(submodels).expect("serialises"),
        &mut out,
    );
    out
}

/// No field a sector's catalog entry classifies non-public appears in a public
/// AAS projection — asserted field-by-field from the catalog, for every sector
/// with a case here, not from a list written by hand.
///
/// The mappers pre-date the disclosure seam and, handed a whole passport,
/// emitted every field they knew of — eight of battery's ten non-public ones
/// among them, including the one that leaked publicly in 0.10.0. Nothing served
/// the projection over HTTP, so it was never a live leak. This is the gate that
/// stops it becoming one, and it is driven from the sector manifests so that
/// reclassifying a field is covered the day it changes.
#[test]
fn public_aas_projection_emits_no_non_public_field() {
    let catalog = dpp_domain::SectorCatalog::new();
    let mut checked_any_non_public = false;

    for (sector, data, version, _) in all_sector_cases() {
        let key = sector.catalog_key().to_owned();
        let non_public: Vec<String> = catalog
            .get(&key)
            .map(|d| {
                d.disclosure
                    .iter()
                    .filter(|(_, class)| **class != dpp_domain::Disclosure::Public)
                    .map(|(field, _)| field.clone())
                    .collect()
            })
            .unwrap_or_default();
        if non_public.is_empty() {
            continue;
        }
        checked_any_non_public = true;

        let passport = base(sector, data, version);
        let (_, submodels) =
            build_aas_from_passport(&passport, VALID_GTIN, dpp_domain::Audience::Public)
                .expect("a public projection is buildable");
        let emitted = emitted_id_shorts(&submodels);

        for field in &non_public {
            assert!(
                !emitted.contains(field),
                "non-public field '{field}' of sector '{key}' appears in the PUBLIC AAS projection"
            );
        }
    }

    assert!(
        checked_any_non_public,
        "guard: at least one sector must declare non-public fields, or this gate proves nothing"
    );
}

/// The gate above is not satisfied by emitting nothing: a public projection
/// still carries its public fields. One that leaked nothing because it
/// contained nothing would pass and be worthless.
#[test]
fn public_aas_projection_still_carries_public_fields() {
    for (sector, data, version, _) in all_sector_cases() {
        let key = sector.catalog_key().to_owned();
        let passport = base(sector, data, version);
        let (_, submodels) =
            build_aas_from_passport(&passport, VALID_GTIN, dpp_domain::Audience::Public)
                .expect("buildable");
        assert!(
            !emitted_id_shorts(&submodels).is_empty(),
            "sector '{key}': the public projection carries no fields at all"
        );
    }
}
