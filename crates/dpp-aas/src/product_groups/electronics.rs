use dpp_domain::product_group::ElectronicsData;

use crate::model::{AasReference, AasSemId, AasSubmodel, AasSubmodelElement};
use crate::property::{
    boolean_property, double_property, enum_wire_str, integer_property, string_property,
};
use crate::semantic_ids;

pub(super) fn build_electronics_submodel(e: &ElectronicsData, passport_id: &str) -> AasSubmodel {
    let mut elements = vec![
        string_property("gtin", e.gtin.as_str(), None),
        string_property("productCategory", &enum_wire_str(&e.product_category), None),
        string_property(
            "energyEfficiencyClass",
            &enum_wire_str(&e.energy_efficiency_class),
            None,
        ),
        double_property(
            "co2ePerUnitKg",
            e.co2e_per_unit_kg,
            Some(semantic_ids::CO2E_PER_UNIT),
        ),
    ];

    if let Some(ref rs) = e.repairability_score {
        elements.push(double_property(
            "repairabilityScore",
            rs.overall,
            Some(semantic_ids::REPAIRABILITY_SCORE),
        ));
    }
    if let Some(v) = e.spare_parts_available {
        elements.push(boolean_property("sparePartsAvailable", v, None));
    }
    if let Some(v) = e.rohs_compliant {
        elements.push(boolean_property("rohsCompliant", v, None));
    }
    if let Some(v) = e.recycled_content_pct {
        elements.push(double_property("recycledContentPct", v, None));
    }
    if let Some(v) = e.standby_power_w {
        elements.push(double_property("standbyPowerW", v, None));
    }
    if let Some(v) = e.expected_lifetime_years {
        elements.push(integer_property("expectedLifetimeYears", v as i64, None));
    }
    if let Some(ref dt) = e.firmware_update_until {
        elements.push(string_property(
            "firmwareUpdateUntil",
            &dt.to_rfc3339(),
            None,
        ));
    }
    if let Some(ref url) = e.repair_manual_url {
        elements.push(AasSubmodelElement::ReferenceElement(
            AasReference::external("repairManualUrl", url),
        ));
    }
    if let Some(ref url) = e.disassembly_instructions_url {
        elements.push(AasSubmodelElement::ReferenceElement(
            AasReference::external("disassemblyInstructionsUrl", url),
        ));
    }

    AasSubmodel {
        id: format!("urn:odal-node:dpp:{passport_id}:electronics-product-data"),
        id_short: "ElectronicsProductData".into(),
        model_type: "Submodel".into(),
        kind: "Instance".into(),
        semantic_id: Some(AasSemId::external(semantic_ids::ELECTRONICS_PRODUCT_DATA)),
        submodel_elements: elements,
    }
}
