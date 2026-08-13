use dpp_domain::BatteryData;

use crate::model::{AasCollection, AasReference, AasSemId, AasSubmodel, AasSubmodelElement};
use crate::property::{
    double_property, enum_wire_str, integer_property, opt_enum_wire_str, string_property,
};
use crate::semantic_ids;

pub(super) fn build_battery_submodel(b: &BatteryData, passport_id: &str) -> AasSubmodel {
    let mut elements = vec![
        string_property("gtin", b.gtin.as_str(), None),
        string_property("batteryChemistry", b.battery_chemistry.wire_str(), None),
        double_property("nominalVoltageV", b.nominal_voltage_v, None),
        double_property("nominalCapacityAh", b.nominal_capacity_ah, None),
        double_property(
            "co2ePerUnitKg",
            b.co2e_per_unit_kg,
            Some(semantic_ids::CO2E_PER_UNIT),
        ),
        string_property("batteryType", &enum_wire_str(&b.battery_type), None),
    ];

    macro_rules! push_opt_double {
        ($opt:expr, $id:literal) => {
            if let Some(v) = $opt {
                elements.push(double_property($id, v, None));
            }
        };
    }
    macro_rules! push_opt_str {
        ($opt:expr, $id:literal) => {
            if let Some(ref v) = $opt {
                elements.push(string_property($id, v, None));
            }
        };
    }

    if let Some(cycles) = b.expected_lifetime_cycles {
        elements.push(integer_property(
            "expectedLifetimeCycles",
            i64::from(cycles),
            None,
        ));
    }
    if let Some(status) = b.battery_status {
        elements.push(string_property(
            "batteryStatus",
            &enum_wire_str(&status),
            None,
        ));
    }

    push_opt_double!(b.recycled_content_cobalt_pct, "recycledContentCobaltPct");
    push_opt_double!(b.recycled_content_lithium_pct, "recycledContentLithiumPct");
    push_opt_double!(b.recycled_content_nickel_pct, "recycledContentNickelPct");
    push_opt_double!(b.state_of_health_pct, "stateOfHealthPct");
    push_opt_double!(b.rated_capacity_kwh, "ratedCapacityKwh");
    push_opt_double!(b.rated_energy_wh, "ratedEnergyWh");
    push_opt_double!(b.battery_weight_kg, "batteryWeightKg");
    push_opt_double!(
        b.initial_round_trip_efficiency_pct,
        "initialRoundTripEfficiencyPct"
    );
    push_opt_double!(
        b.round_trip_efficiency_at_half_cycle_life_pct,
        "roundTripEfficiencyAtHalfCycleLifePct"
    );
    push_opt_double!(
        b.internal_cell_resistance_mohm,
        "internalCellResistanceMohm"
    );
    push_opt_double!(
        b.internal_pack_resistance_mohm,
        "internalPackResistanceMohm"
    );
    // Annex XIII point 1 technical characteristics.
    //
    // # What this submodel deliberately does not carry
    //
    // Collected here so an omission reads as a decision rather than an
    // oversight — a distinction the absence itself cannot make:
    //
    // - **Document-shaped point 1 items** — `euDeclarationOfConformity`,
    //   `markingInformation`, `wasteBatteryInformation`. This submodel is a
    //   technical-data snapshot, and a reference to a document is not a
    //   property of the asset.
    // - **Annex XIII points 2 and 3** — `componentPartNumbers`,
    //   `sparePartsContacts`, `safetyMeasures`, `testReportResults`. Same
    //   reason: documents and contacts. They are still carried on the passport
    //   and still gated by their disclosure classes; they just do not project.
    // - **Point 4(d) `usageHistory`** — see the note beside
    //   `dynamicPerformance` below.
    //
    // `expectedLifetime` and `stateOfHealth` are in none of those categories.
    // Both are measurements and both belong in a technical snapshot; they are
    // absent because nobody has mapped them, which is a gap rather than a
    // decision, and the only one left in this mapper.
    push_opt_double!(b.minimal_voltage_v, "minimalVoltageV");
    push_opt_double!(b.maximum_voltage_v, "maximumVoltageV");
    push_opt_double!(b.original_power_capability_w, "originalPowerCapabilityW");
    push_opt_double!(b.power_limit_min_w, "powerLimitMinW");
    push_opt_double!(b.power_limit_max_w, "powerLimitMaxW");
    push_opt_double!(b.renewable_content_pct, "renewableContentPct");
    push_opt_double!(
        b.capacity_threshold_for_exhaustion_pct,
        "capacityThresholdForExhaustionPct"
    );
    push_opt_double!(b.cycle_life_test_c_rate, "cycleLifeTestCRate");
    push_opt_double!(b.operating_temp_min_c, "operatingTempMinC");
    push_opt_double!(b.operating_temp_max_c, "operatingTempMaxC");
    push_opt_double!(b.recycled_content_lead_pct, "recycledContentLeadPct");
    // The class never travels without the ruleset that produced it. Its own
    // type says why: "the same label denotes different thresholds under
    // different revisions of the scale, and Art. 7(2) requires those thresholds
    // to be reviewed every three years". A bare `B` in an exported AAS is not a
    // weaker claim than a qualified one — it is an unfalsifiable one, because a
    // consumer cannot tell which scale it was measured against.
    if let Some(s) = opt_enum_wire_str(&b.carbon_footprint_class) {
        elements.push(string_property("carbonFootprintClass", &s, None));
    }
    push_opt_str!(
        b.carbon_footprint_class_ruleset_id,
        "carbonFootprintClassRulesetId"
    );
    push_opt_str!(
        b.carbon_footprint_class_ruleset_version,
        "carbonFootprintClassRulesetVersion"
    );
    push_opt_str!(b.soh_methodology, "sohMethodology");

    // Annex VI Part A identity — points 2 (the battery-identifying half), 3 and
    // 4, plus the Art. 77(3) unique identifier. They project here rather than
    // into `ProductIdentification` because that submodel is built from the
    // passport envelope and never sees sector data; this is the only place they
    // can reach a consumer at all.
    push_opt_str!(b.battery_model_id, "batteryModelId");
    push_opt_str!(b.battery_passport_number, "batteryPassportNumber");
    push_opt_str!(b.manufacturing_place, "manufacturingPlace");
    if let Some(when) = b.manufacturing_date {
        elements.push(string_property(
            "manufacturingDate",
            &when.to_rfc3339(),
            None,
        ));
    }

    // Annex VI Part A points 8 and 9, and the Annex XIII point 1 conditions
    // that qualify the figures above. All are properties of the asset rather
    // than documents about it, so the exclusion stated further up does not
    // reach them.
    push_opt_str!(b.usable_extinguishing_agent, "usableExtinguishingAgent");
    push_opt_str!(
        b.expected_lifetime_reference_test,
        "expectedLifetimeReferenceTest"
    );
    push_opt_str!(
        b.not_in_use_temperature_reference_test,
        "notInUseTemperatureReferenceTest"
    );
    if let Some(ref sym) = b.hazard_symbol {
        elements.push(string_property("hazardSymbol", &enum_wire_str(sym), None));
    }
    if let Some(months) = b.commercial_warranty_period_months {
        elements.push(integer_property(
            "commercialWarrantyPeriodMonths",
            i64::from(months),
            None,
        ));
    }
    if let Some(year) = b.recycled_content_reporting_year {
        elements.push(integer_property(
            "recycledContentReportingYear",
            i64::from(year),
            None,
        ));
    }
    if let Some(date) = b.placed_on_market_date {
        elements.push(string_property(
            "placedOnMarketDate",
            &date.to_string(),
            None,
        ));
    }

    // Annex XIII point 1(h), (i) and (l) each attach a temperature range to a
    // figure. The range is two numbers that mean nothing apart, so it projects
    // as a collection rather than two loose properties named by convention.
    for (label, opt_range) in [
        ("voltageTemperatureRange", b.voltage_temperature_range),
        ("powerTemperatureRange", b.power_temperature_range),
        ("notInUseTemperatureRange", b.not_in_use_temperature_range),
    ] {
        if let Some(range) = opt_range {
            elements.push(AasSubmodelElement::SubmodelElementCollection(
                AasCollection {
                    id_short: label.to_owned(),
                    value: vec![
                        double_property("minC", range.min_c, None),
                        double_property("maxC", range.max_c, None),
                    ],
                    semantic_id: None,
                },
            ));
        }
    }

    // Annex VI Part A point 8 — the substances themselves, not a document about
    // them, and the one place a public reader learns what is in the battery
    // beyond its chemistry.
    if let Some(ref subs) = b.hazardous_substances {
        let items = subs
            .iter()
            .enumerate()
            .map(|(i, hs)| {
                let mut hs_elems = vec![string_property("name", &hs.name, None)];
                if let Some(ref cas) = hs.cas_number {
                    hs_elems.push(string_property("casNumber", cas, None));
                }
                if let Some(pct) = hs.concentration_pct {
                    hs_elems.push(double_property("concentrationPct", pct, None));
                }
                AasSubmodelElement::SubmodelElementCollection(AasCollection {
                    id_short: format!("hazardousSubstance_{i}"),
                    value: hs_elems,
                    semantic_id: None,
                })
            })
            .collect();
        elements.push(AasSubmodelElement::SubmodelElementCollection(
            AasCollection {
                id_short: "hazardousSubstances".to_owned(),
                value: items,
                semantic_id: None,
            },
        ));
    }

    if let Some(ref url) = b.due_diligence_url {
        elements.push(AasSubmodelElement::ReferenceElement(
            AasReference::external("dueDiligenceUrl", url),
        ));
    }
    if let Some(ref url) = b.disassembly_instructions_url {
        elements.push(AasSubmodelElement::ReferenceElement(
            AasReference::external("disassemblyInstructionsUrl", url),
        ));
    }

    for (label, opt_comps) in [
        ("cathodeMaterial", &b.cathode_material),
        ("anodeMaterial", &b.anode_material),
        ("electrolyteMaterial", &b.electrolyte_material),
    ] {
        if let Some(comps) = opt_comps {
            let items = comps
                .iter()
                .enumerate()
                .map(|(i, mc)| {
                    let mut mc_elems = vec![
                        string_property("name", &mc.name, None),
                        double_property("weightPct", mc.weight_pct, None),
                    ];
                    if let Some(ref cas) = mc.cas_number {
                        mc_elems.push(string_property("casNumber", cas, None));
                    }
                    AasSubmodelElement::SubmodelElementCollection(AasCollection {
                        id_short: format!("{label}_{i}"),
                        value: mc_elems,
                        semantic_id: None,
                    })
                })
                .collect();
            elements.push(AasSubmodelElement::SubmodelElementCollection(
                AasCollection {
                    id_short: label.to_owned(),
                    value: items,
                    semantic_id: None,
                },
            ));
        }
    }

    // Annex XIII point 4(a), measured per battery. Nested rather than flattened
    // for the same reason the domain type is: a declared model figure and a
    // measured one are different claims and must not sit side by side under
    // near-identical names.
    //
    // Point 4(d) — `usageHistory` — is deliberately **not** projected. It is a
    // set of time series, and this submodel is a technical-data snapshot; a
    // consumer wanting use history wants the passport, not an AAS property.
    if let Some(ref dp) = b.dynamic_performance {
        let mut dyn_elems = Vec::new();
        for (label, value) in [
            ("ratedCapacityAh", dp.rated_capacity_ah),
            ("capacityFadePct", dp.capacity_fade_pct),
            ("powerW", dp.power_w),
            ("powerFadePct", dp.power_fade_pct),
            ("internalResistanceMohm", dp.internal_resistance_mohm),
            (
                "internalResistanceIncreasePct",
                dp.internal_resistance_increase_pct,
            ),
            ("roundTripEfficiencyPct", dp.round_trip_efficiency_pct),
            (
                "roundTripEfficiencyFadePct",
                dp.round_trip_efficiency_fade_pct,
            ),
            ("expectedLifetimeYears", dp.expected_lifetime_years),
        ] {
            if let Some(v) = value {
                dyn_elems.push(double_property(label, v, None));
            }
        }
        if let Some(cycles) = dp.expected_lifetime_cycles {
            dyn_elems.push(integer_property(
                "expectedLifetimeCycles",
                i64::from(cycles),
                None,
            ));
        }
        if !dyn_elems.is_empty() {
            elements.push(AasSubmodelElement::SubmodelElementCollection(
                AasCollection {
                    id_short: "dynamicPerformance".to_owned(),
                    value: dyn_elems,
                    semantic_id: None,
                },
            ));
        }
    }

    if let Some(ref crms) = b.critical_raw_materials {
        let items = crms
            .iter()
            .enumerate()
            .map(|(i, crm)| {
                let mut crm_elems = vec![string_property("name", &crm.name, None)];
                if let Some(ref cas) = crm.cas_number {
                    crm_elems.push(string_property("casNumber", cas, None));
                }
                if let Some(wg) = crm.weight_grams {
                    crm_elems.push(double_property("weightGrams", wg, None));
                }
                if let Some(ref country) = crm.country_of_origin {
                    crm_elems.push(string_property("countryOfOrigin", country, None));
                }
                AasSubmodelElement::SubmodelElementCollection(AasCollection {
                    id_short: format!("criticalRawMaterial_{i}"),
                    value: crm_elems,
                    semantic_id: None,
                })
            })
            .collect();
        elements.push(AasSubmodelElement::SubmodelElementCollection(
            AasCollection {
                id_short: "criticalRawMaterials".to_owned(),
                value: items,
                semantic_id: None,
            },
        ));
    }

    AasSubmodel {
        id: format!("urn:odal-node:dpp:{passport_id}:battery-technical-data"),
        id_short: "BatteryTechnicalData".into(),
        model_type: "Submodel".into(),
        kind: "Instance".into(),
        semantic_id: Some(AasSemId::external(semantic_ids::BATTERY_TECHNICAL_DATA)),
        submodel_elements: elements,
    }
}
