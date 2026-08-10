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
        integer_property(
            "expectedLifetimeCycles",
            b.expected_lifetime_cycles as i64,
            None,
        ),
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

    push_opt_double!(b.recycled_content_cobalt_pct, "recycledContentCobaltPct");
    push_opt_double!(b.recycled_content_lithium_pct, "recycledContentLithiumPct");
    push_opt_double!(b.recycled_content_nickel_pct, "recycledContentNickelPct");
    push_opt_double!(b.state_of_health_pct, "stateOfHealthPct");
    push_opt_double!(b.rated_capacity_kwh, "ratedCapacityKwh");
    push_opt_double!(b.rated_energy_wh, "ratedEnergyWh");
    push_opt_double!(b.battery_weight_kg, "batteryWeightKg");
    push_opt_double!(b.round_trip_efficiency_pct, "roundTripEfficiencyPct");
    push_opt_double!(b.internal_resistance_mohm, "internalResistanceMohm");
    push_opt_double!(b.operating_temp_min_c, "operatingTempMinC");
    push_opt_double!(b.operating_temp_max_c, "operatingTempMaxC");
    push_opt_double!(b.recycled_content_lead_pct, "recycledContentLeadPct");
    if let Some(s) = opt_enum_wire_str(&b.carbon_footprint_class) {
        elements.push(string_property("carbonFootprintClass", &s, None));
    }
    push_opt_str!(b.soh_methodology, "sohMethodology");

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
