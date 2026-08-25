use super::semantic_ids;

/// Metadata for a single AAS submodel template binding.
#[derive(Debug, Clone, PartialEq)]
pub struct SubmodelTemplate {
    /// Catalog product group key this template applies to, e.g. `"battery"`.
    pub product_group_key: &'static str,
    /// Semantic ID (IDTA URN, Catena-X URN, ECLASS IRDI, or odal-node placeholder).
    pub semantic_id: &'static str,
    /// Human-readable version string (from the source template / standard).
    pub version: &'static str,
}

impl SubmodelTemplate {
    /// Whether this binding still names *our* concept rather than a ratified
    /// third-party template. Gate conformance claims on this.
    ///
    /// Derived from the identifier rather than stored beside it, and that is the
    /// point. It was a hand-maintained `bool`, and when battery's constant was
    /// reverted from the Catena-X aspect model back into our own namespace on
    /// 2026-07-29, the flag stayed `false` and the `version` stayed `"6.0.0"` —
    /// the aspect model's version, on an identifier ending `:1.0`. So a template
    /// carrying a placeholder identifier described itself as ratified, which is
    /// the one thing this field exists to prevent.
    ///
    /// A value that can be derived from another must be, or the two will
    /// eventually disagree and the wrong one will be the one somebody trusts.
    pub fn is_placeholder(&self) -> bool {
        dpp_vocab::is_own(self.semantic_id)
    }
}

static SUBMODEL_TEMPLATES: &[SubmodelTemplate] = &[
    SubmodelTemplate {
        product_group_key: "battery",
        semantic_id: semantic_ids::BATTERY_TECHNICAL_DATA,
        version: "1.0",
    },
    SubmodelTemplate {
        product_group_key: "textile",
        semantic_id: semantic_ids::TEXTILE_MATERIAL,
        version: "1.0",
    },
    SubmodelTemplate {
        product_group_key: "electronics",
        semantic_id: semantic_ids::ELECTRONICS_PRODUCT_DATA,
        version: "1.0",
    },
    SubmodelTemplate {
        product_group_key: "steel",
        semantic_id: semantic_ids::STEEL_PRODUCT_DATA,
        version: "1.0",
    },
    SubmodelTemplate {
        product_group_key: "construction",
        semantic_id: semantic_ids::CONSTRUCTION_PRODUCT_DATA,
        version: "1.0",
    },
    SubmodelTemplate {
        product_group_key: "tyre",
        semantic_id: semantic_ids::TYRE_PRODUCT_DATA,
        version: "1.0",
    },
    SubmodelTemplate {
        product_group_key: "toy",
        semantic_id: semantic_ids::TOY_PRODUCT_DATA,
        version: "1.0",
    },
    SubmodelTemplate {
        product_group_key: "aluminium",
        semantic_id: semantic_ids::ALUMINIUM_PRODUCT_DATA,
        version: "1.0",
    },
    SubmodelTemplate {
        product_group_key: "furniture",
        semantic_id: semantic_ids::FURNITURE_PRODUCT_DATA,
        version: "1.0",
    },
    SubmodelTemplate {
        product_group_key: "detergent",
        semantic_id: semantic_ids::DETERGENT_PRODUCT_DATA,
        version: "1.0",
    },
    SubmodelTemplate {
        product_group_key: "unsold-goods",
        semantic_id: semantic_ids::UNSOLD_GOODS_REPORT,
        version: "1.0",
    },
];

/// Look up the AAS submodel template binding for a catalog product group key.
///
/// Returns `None` for product groups that don't yet have a dedicated AAS template.
/// Returns `Some(t)` where `t.is_placeholder()` is `true` when the semantic ID
/// is a draft Odal Node placeholder, not a ratified IDTA standard.
pub fn product_group_submodel_template(
    product_group_key: &str,
) -> Option<&'static SubmodelTemplate> {
    SUBMODEL_TEMPLATES
        .iter()
        .find(|t| t.product_group_key == product_group_key)
}

/// Returns every product group template whose semantic ID is still a placeholder.
///
/// Use this in CI to gate placeholder IDs from being promoted as conformant.
pub fn placeholder_templates() -> impl Iterator<Item = &'static SubmodelTemplate> {
    SUBMODEL_TEMPLATES.iter().filter(|t| t.is_placeholder())
}
