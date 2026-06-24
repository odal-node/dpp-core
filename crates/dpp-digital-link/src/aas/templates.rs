use super::semantic_ids;

/// Metadata for a single AAS submodel template binding.
#[derive(Debug, Clone, PartialEq)]
pub struct SubmodelTemplate {
    /// Catalog sector key this template applies to, e.g. `"battery"`.
    pub sector_key: &'static str,
    /// Semantic ID (IDTA URN, Catena-X URN, ECLASS IRDI, or odal-node placeholder).
    pub semantic_id: &'static str,
    /// Human-readable version string (from the source template / standard).
    pub version: &'static str,
    /// `true` when the semantic ID is a placeholder (`urn:odal-node:…`) waiting for
    /// an official IDTA or other standard template. Gate these from claiming
    /// conformance with the AAS Interoperability Specification.
    pub is_placeholder: bool,
}

static SUBMODEL_TEMPLATES: &[SubmodelTemplate] = &[
    SubmodelTemplate {
        sector_key: "battery",
        semantic_id: semantic_ids::BATTERY_TECHNICAL_DATA,
        version: "6.0.0",
        is_placeholder: false,
    },
    SubmodelTemplate {
        sector_key: "textile",
        semantic_id: semantic_ids::TEXTILE_MATERIAL,
        version: "1.0",
        is_placeholder: true,
    },
    SubmodelTemplate {
        sector_key: "electronics",
        semantic_id: semantic_ids::ELECTRONICS_PRODUCT_DATA,
        version: "1.0",
        is_placeholder: true,
    },
    SubmodelTemplate {
        sector_key: "steel",
        semantic_id: semantic_ids::STEEL_PRODUCT_DATA,
        version: "1.0",
        is_placeholder: true,
    },
    SubmodelTemplate {
        sector_key: "construction",
        semantic_id: semantic_ids::CONSTRUCTION_PRODUCT_DATA,
        version: "1.0",
        is_placeholder: true,
    },
    SubmodelTemplate {
        sector_key: "tyre",
        semantic_id: semantic_ids::TYRE_PRODUCT_DATA,
        version: "1.0",
        is_placeholder: true,
    },
    SubmodelTemplate {
        sector_key: "toy",
        semantic_id: semantic_ids::TOY_PRODUCT_DATA,
        version: "1.0",
        is_placeholder: true,
    },
    SubmodelTemplate {
        sector_key: "aluminium",
        semantic_id: semantic_ids::ALUMINIUM_PRODUCT_DATA,
        version: "1.0",
        is_placeholder: true,
    },
    SubmodelTemplate {
        sector_key: "furniture",
        semantic_id: semantic_ids::FURNITURE_PRODUCT_DATA,
        version: "1.0",
        is_placeholder: true,
    },
    SubmodelTemplate {
        sector_key: "detergent",
        semantic_id: semantic_ids::DETERGENT_PRODUCT_DATA,
        version: "1.0",
        is_placeholder: true,
    },
    SubmodelTemplate {
        sector_key: "textile-unsold-goods",
        semantic_id: semantic_ids::UNSOLD_GOODS_REPORT,
        version: "1.0",
        is_placeholder: true,
    },
];

/// Look up the AAS submodel template binding for a catalog sector key.
///
/// Returns `None` for sectors that don't yet have a dedicated AAS template.
/// Returns `Some(t)` where `t.is_placeholder == true` when the semantic ID is
/// a draft Odal Node placeholder, not a ratified IDTA standard.
pub fn sector_submodel_template(sector_key: &str) -> Option<&'static SubmodelTemplate> {
    SUBMODEL_TEMPLATES
        .iter()
        .find(|t| t.sector_key == sector_key)
}

/// Returns every sector template whose semantic ID is still a placeholder.
///
/// Use this in CI to gate placeholder IDs from being promoted as conformant.
pub fn placeholder_templates() -> impl Iterator<Item = &'static SubmodelTemplate> {
    SUBMODEL_TEMPLATES.iter().filter(|t| t.is_placeholder)
}
