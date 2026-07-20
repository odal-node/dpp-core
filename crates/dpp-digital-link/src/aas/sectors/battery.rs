use dpp_domain::BatteryData;

use crate::aas::model::{AasCollection, AasReference, AasSemId, AasSubmodel, AasSubmodelElement};
use crate::aas::property::{
    double_property, enum_wire_str, integer_property, opt_enum_wire_str, string_property,
};
use crate::aas::semantic_ids;

pub(super) fn build_battery_submodel(b: &BatteryData, passport_id: &str) -> AasSubmodel {
    let chemistry_str = enum_wire_str(&b.battery_chemistry);
    let mut elements = vec![
        string_property("gtin", b.gtin.as_str(), None, None),
        string_property("batteryChemistry", &chemistry_str, None, None),
        double_property("nominalVoltageV", b.nominal_voltage_v, None, Some("V")),
        double_property("nominalCapacityAh", b.nominal_capacity_ah, None, Some("Ah")),
        integer_property(
            "expectedLifetimeCycles",
            b.expected_lifetime_cycles as i64,
            None,
            None,
        ),
        double_property(
            "co2ePerUnitKg",
            b.co2e_per_unit_kg,
            Some(semantic_ids::CARBON_FOOTPRINT),
            Some("kgCO2e"),
        ),
    ];

    macro_rules! push_opt_double {
        ($opt:expr, $id:literal, $unit:expr) => {
            if let Some(v) = $opt {
                elements.push(double_property($id, v, None, $unit));
            }
        };
    }
    macro_rules! push_opt_str {
        ($opt:expr, $id:literal) => {
            if let Some(ref v) = $opt {
                elements.push(string_property($id, v, None, None));
            }
        };
    }

    push_opt_double!(
        b.recycled_content_cobalt_pct,
        "recycledContentCobaltPct",
        Some("%")
    );
    push_opt_double!(
        b.recycled_content_lithium_pct,
        "recycledContentLithiumPct",
        Some("%")
    );
    push_opt_double!(
        b.recycled_content_nickel_pct,
        "recycledContentNickelPct",
        Some("%")
    );
    push_opt_double!(b.state_of_health_pct, "stateOfHealthPct", Some("%"));
    push_opt_double!(b.rated_capacity_kwh, "ratedCapacityKwh", Some("kWh"));
    push_opt_double!(b.rated_energy_wh, "ratedEnergyWh", Some("Wh"));
    push_opt_double!(b.battery_weight_kg, "batteryWeightKg", Some("kg"));
    push_opt_double!(
        b.round_trip_efficiency_pct,
        "roundTripEfficiencyPct",
        Some("%")
    );
    push_opt_double!(
        b.internal_resistance_mohm,
        "internalResistanceMohm",
        Some("mΩ")
    );
    push_opt_double!(b.operating_temp_min_c, "operatingTempMinC", Some("°C"));
    push_opt_double!(b.operating_temp_max_c, "operatingTempMaxC", Some("°C"));
    push_opt_double!(
        b.recycled_content_lead_pct,
        "recycledContentLeadPct",
        Some("%")
    );
    if let Some(s) = opt_enum_wire_str(&b.carbon_footprint_class) {
        elements.push(string_property("carbonFootprintClass", &s, None, None));
    }
    if let Some(s) = opt_enum_wire_str(&b.battery_type) {
        elements.push(string_property("batteryType", &s, None, None));
    }
    push_opt_str!(b.soh_methodology, "sohMethodology");

    if let Some(ref url) = b.due_diligence_url {
        elements.push(AasSubmodelElement::Reference(AasReference {
            id_short: "dueDigiligenceUrl".into(),
            value: url.clone(),
            semantic_id: None,
        }));
    }
    if let Some(ref url) = b.disassembly_instructions_url {
        elements.push(AasSubmodelElement::Reference(AasReference {
            id_short: "disassemblyInstructionsUrl".into(),
            value: url.clone(),
            semantic_id: None,
        }));
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
                        string_property("name", &mc.name, None, None),
                        double_property("weightPct", mc.weight_pct, None, Some("%")),
                    ];
                    if let Some(ref cas) = mc.cas_number {
                        mc_elems.push(string_property("casNumber", cas, None, None));
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

    if let Some(ref crms) = b.critical_raw_materials {
        let items = crms
            .iter()
            .enumerate()
            .map(|(i, crm)| {
                let mut crm_elems = vec![string_property("name", &crm.name, None, None)];
                if let Some(ref cas) = crm.cas_number {
                    crm_elems.push(string_property("casNumber", cas, None, None));
                }
                if let Some(wg) = crm.weight_grams {
                    crm_elems.push(double_property("weightGrams", wg, None, Some("g")));
                }
                if let Some(ref country) = crm.country_of_origin {
                    crm_elems.push(string_property("countryOfOrigin", country, None, None));
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
