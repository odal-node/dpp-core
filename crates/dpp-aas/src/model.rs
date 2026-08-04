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

    /// Build a model reference to a `Submodel` by its identifier.
    ///
    /// `ModelReference` is the reference type for a pointer at another element
    /// of the metamodel — which is what a shell's `submodels` list holds. An
    /// `ExternalReference` here would assert the target is outside the AAS
    /// information model, which is the opposite of what it is.
    pub fn submodel(id: &str) -> Self {
        Self {
            ref_type: "ModelReference".into(),
            keys: vec![AasSemIdKey {
                key_type: "Submodel".into(),
                value: id.to_owned(),
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
    /// Omitted when empty: the schema constrains `value` to `minItems: 1`, so
    /// an empty array is invalid where an absent one is fine.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
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
    /// Target, as a `Reference`. IDTA AAS Part 1 §5.3.7.13 types
    /// `ReferenceElement.value` as a `Reference`, not a bare string, so a URL
    /// is carried as an `ExternalReference` with one `GlobalReference` key.
    pub value: AasSemId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_id: Option<AasSemId>,
}

impl AasReference {
    /// A reference element pointing at an external URI.
    pub fn external(id_short: &str, uri: &str) -> Self {
        Self {
            id_short: id_short.to_owned(),
            value: AasSemId::external(uri),
            semantic_id: None,
        }
    }
}

/// AAS SubmodelElement — a property, collection, or external reference.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "modelType")]
pub enum AasSubmodelElement {
    Property(AasProperty),
    SubmodelElementCollection(AasCollection),
    /// Named for the metamodel class, because the variant name *is* the wire
    /// `modelType`. `Reference` is a different thing in AAS — the pointer type
    /// carried inside this element — and is not a `SubmodelElement` at all, so
    /// emitting it here produced a document no AAS parser would accept.
    ReferenceElement(AasReference),
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
    /// Omitted when empty, for the same `minItems: 1` reason as
    /// [`AasCollection::value`]. A submodel whose every field was absent (or
    /// masked away for this audience) is a legitimate outcome; an empty array
    /// on the wire is not.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
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
    /// The **only** required member of `AssetInformation` in the metamodel.
    /// `"Instance"` for a passport: it describes one manufactured item or
    /// batch, never a product type definition.
    pub asset_kind: String,
    pub global_asset_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub specific_asset_ids: Vec<SpecificAssetId>,
}

/// AAS Shell — the top-level container for a product's digital twin.
///
/// Holds the asset identification and references to submodels. Submodels are
/// served alongside the shell as `Vec<AasSubmodel>` from [`build_aas_from_passport`](super::build_aas_from_passport)
/// and would be exposed from separate API endpoints in a running AAS server
/// (`/shells/{aasId}` vs. `/submodels/{submodelId}`).
///
/// **No `kind` here.** `kind` comes from `HasKind`, which `Submodel` composes
/// and `AssetAdministrationShell` does not — the shell is
/// `Identifiable` + `HasDataSpecification` + `derivedFrom`, `assetInformation`,
/// `submodels`. We emitted one anyway for several releases, and no gate could
/// have told us: IDTA's schema sets `additionalProperties` nowhere, so a member
/// that is not part of a class validates in silence. A strict loader is the
/// thing that would have rejected it, which is why it does not belong here even
/// though the schema never complained.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AasShell {
    pub id: String,
    pub id_short: String,
    /// IDTA AAS Part 2 §5.2.4: always `"AssetAdministrationShell"`.
    pub model_type: String,
    pub asset_information: AssetInformation,
    /// `ModelReference`s to this shell's submodels, per
    /// [`AasSemId::submodel`]. Omitted when empty (`minItems: 1`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub submodels: Vec<AasSemId>,
}

// ─── Environment ──────────────────────────────────────────────────────────────

/// IDTA AAS `Environment` — the self-contained document form, carrying shells
/// and submodels together in one payload.
///
/// This is the *file* shape, not the API shape. A running AAS server serves
/// [`AasShell`] and [`AasSubmodel`] from separate endpoints
/// (`/shells/{aasId}`, `/submodels/{submodelId}`); an `Environment` is what you
/// serve when a caller wants the whole twin in one response, and it is what an
/// AASX package contains.
///
/// It lives here rather than in a consumer because there must be exactly one
/// serialisation of it: an HTTP door and an AASX package that each built their
/// own would drift, and the drift would be invisible until a partner's tooling
/// disagreed with ours.
///
/// **Schema-valid, not conformance-certified.** Every Environment this crate
/// builds is validated in CI against the vendored `IDTA-01001-3-0-1` JSON
/// Schema (see `dpp-tests/fixtures/aas/`). That establishes metamodel validity
/// and nothing more: it is not a claim of IDTA conformance, which would need
/// IDTA's own test engine, and it says nothing about whether a submodel matches
/// a published submodel template.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AasEnvironment {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub asset_administration_shells: Vec<AasShell>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub submodels: Vec<AasSubmodel>,
    /// Always empty: this crate coins no concept descriptions. Omitted from the
    /// wire rather than emitted as `[]`, which the schema rejects
    /// (`minItems: 1`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub concept_descriptions: Vec<serde_json::Value>,
}
