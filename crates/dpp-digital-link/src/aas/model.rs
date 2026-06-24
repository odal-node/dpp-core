use serde::{Deserialize, Serialize};

// ─── Semantic ID reference ────────────────────────────────────────────────────

/// IDTA AAS Part 1 §5.3.11 Key — one segment of a semantic identifier reference.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AasSemIdKey {
    #[serde(rename = "type")]
    pub key_type: String,
    pub value: String,
}

/// IDTA AAS Part 1 §5.3.11 Reference — typed container for semantic identifiers.
///
/// External semantic IDs (ECLASS IRDIs, IDTA URNs, Catena-X URNs) use
/// `type = "ExternalReference"` with a single `GlobalReference` key.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AasSemId {
    #[serde(rename = "type")]
    pub ref_type: String,
    pub keys: Vec<AasSemIdKey>,
}

impl AasSemId {
    /// Build an external semantic ID reference from a URI or IRDI string.
    pub fn external(value: &str) -> Self {
        Self {
            ref_type: "ExternalReference".into(),
            keys: vec![AasSemIdKey {
                key_type: "GlobalReference".into(),
                value: value.to_owned(),
            }],
        }
    }
}

// ─── Submodel element types ───────────────────────────────────────────────────

/// AAS data type for `Property` values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AasDataType {
    #[serde(rename = "xs:string")]
    String,
    #[serde(rename = "xs:double")]
    Double,
    #[serde(rename = "xs:integer")]
    Integer,
    #[serde(rename = "xs:boolean")]
    Boolean,
    #[serde(rename = "xs:dateTime")]
    DateTime,
}

/// An AAS Property — a single leaf-level value.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AasProperty {
    pub id_short: String,
    pub value_type: AasDataType,
    /// Value serialised as a string (AAS convention for all types).
    pub value: String,
    /// Physical unit of the value, e.g. `"kgCO2e"`, `"kg"`, `"V"`, `"%"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// Semantic identifier per IDTA AAS Part 1 §5.3.11 Reference type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_id: Option<AasSemId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// An AAS SubmodelElementCollection — a named group of elements.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AasCollection {
    pub id_short: String,
    pub value: Vec<AasSubmodelElement>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_id: Option<AasSemId>,
}

/// An AAS ReferenceElement — an external link (URL/URN).
///
/// Used for repair manuals, due-diligence documents, disassembly instructions,
/// and other external resources linked from DPP fields.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AasReference {
    pub id_short: String,
    /// Target URI or URN.
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_id: Option<AasSemId>,
}

/// AAS SubmodelElement — a property, collection, or external reference.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "modelType")]
pub enum AasSubmodelElement {
    Property(AasProperty),
    SubmodelElementCollection(AasCollection),
    Reference(AasReference),
}

/// An AAS Submodel — one named grouping of product data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AasSubmodel {
    pub id: String,
    pub id_short: String,
    /// IDTA AAS Part 2 §5.2.4: `"Submodel"` for all AAS submodels.
    pub model_type: String,
    /// IDTA AAS Part 2 §5.2.4: `"Instance"` for runtime data (not templates).
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_id: Option<AasSemId>,
    pub submodel_elements: Vec<AasSubmodelElement>,
}

// ─── Shell container types ────────────────────────────────────────────────────

/// A name/value specific asset identifier (e.g., `gtin`, `batchId`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SpecificAssetId {
    pub name: String,
    pub value: String,
}

/// AAS asset identification block.
///
/// `global_asset_id` is the canonical URI for the physical product built from
/// the GTIN. `specific_asset_ids` carry GTIN, batch, and serial identifiers
/// for GS1 Digital Link resolution and registry look-up.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AssetInformation {
    pub global_asset_id: String,
    pub specific_asset_ids: Vec<SpecificAssetId>,
}

/// A reference from an `AasShell` to one of its submodels (ID only).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AasSubmodelRef {
    pub id: String,
}

/// AAS Shell — the top-level container for a product's digital twin.
///
/// Holds the asset identification and references to submodels. Submodels are
/// served alongside the shell as `Vec<AasSubmodel>` from [`build_aas_from_passport`](super::build_aas_from_passport)
/// and would be exposed from separate API endpoints in a running AAS server
/// (`/shells/{aasId}` vs. `/submodels/{submodelId}`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AasShell {
    pub id: String,
    pub id_short: String,
    /// IDTA AAS Part 2 §5.2.4: always `"AssetAdministrationShell"`.
    pub model_type: String,
    /// IDTA AAS Part 2 §5.2.4: `"Instance"` for runtime data (not templates).
    pub kind: String,
    pub asset_information: AssetInformation,
    pub submodels: Vec<AasSubmodelRef>,
}
