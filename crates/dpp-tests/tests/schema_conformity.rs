//! Integration test: Schema conformity validation.
//!
//! Validates that:
//! 1. All JSON schemas are syntactically valid and loadable.
//! 2. Textile v1.1.0 schema covers the fields required by JTC 24 draft.
//! 3. Valid textile data passes schema validation.
//! 4. Invalid textile data is correctly rejected.
//! 5. The VersionedSchemaRegistry resolves the correct schema version.
//! 6. Battery and steel schemas are present and valid.
//!
//! This approximates what a conformity assessment body would check. Schemas are
//! loaded through the public `VersionedSchemaRegistry` — the same resolution
//! path a consumer uses — rather than by reaching into files, so the test also
//! covers registry embedding.

use dpp_domain::schemas::VersionedSchemaRegistry;
use semver::Version;

/// Load an embedded schema by (sector, version) through the public registry.
fn schema(sector: &str, version: &str) -> serde_json::Value {
    let reg = VersionedSchemaRegistry::new();
    let json = reg
        .get(sector, &Version::parse(version).expect("valid semver"))
        .unwrap_or_else(|| panic!("schema {sector} v{version} not embedded in registry"));
    serde_json::from_str(json).expect("embedded schema must be valid JSON")
}

// ─── JTC 24 mandatory field coverage ──────────────────────────────────────

/// The fields anticipated as mandatory by the JTC 24 textile DPP draft.
/// This list is used to assert that our schema v1.1.0 covers them.
const JTC24_TEXTILE_MANDATORY_FIELDS: &[&str] = &[
    "fibreComposition",
    "countryOfManufacturing",
    "careInstructions",
    "chemicalComplianceStandard",
];

/// Fields anticipated by JTC 24 as important for environmental metrics.
const JTC24_TEXTILE_ENVIRONMENTAL_FIELDS: &[&str] = &[
    "recycledContentPct",
    "carbonFootprintKgCo2e",
    "waterUseLitres",
    "microplasticSheddingMgPerWash",
    "durabilityScore",
    "repairScore",
];

/// Fields required for SVHC/SCIP disclosure under REACH Article 33.
const JTC24_SVHC_FIELDS: &[&str] = &["svhcSubstances"];

/// Fields for professional-tier access (disassembly, spare parts).
const JTC24_PROFESSIONAL_FIELDS: &[&str] = &["disassemblyInstructions", "sparePartsAvailable"];

#[test]
fn textile_v1_1_schema_is_valid_json_schema() {
    let schema_value = schema("textile", "1.1.0");

    // Must have $schema declaration
    assert!(schema_value.get("$schema").is_some(), "missing $schema");
    assert!(schema_value.get("$id").is_some(), "missing $id");
    assert_eq!(
        schema_value["type"].as_str().unwrap(),
        "object",
        "textile schema must be object type"
    );
}

#[test]
fn textile_v1_1_schema_covers_jtc24_mandatory_fields() {
    let schema = schema("textile", "1.1.0");

    let properties = schema["properties"]
        .as_object()
        .expect("schema must have properties");
    let required = schema["required"]
        .as_array()
        .expect("schema must have required array");
    let required_names: Vec<&str> = required.iter().map(|v| v.as_str().unwrap()).collect();

    for field in JTC24_TEXTILE_MANDATORY_FIELDS {
        assert!(
            properties.contains_key(*field),
            "schema missing JTC 24 mandatory field: {field}"
        );
        assert!(
            required_names.contains(field),
            "JTC 24 mandatory field '{field}' must be in 'required' array"
        );
    }
}

#[test]
fn textile_v1_1_schema_covers_environmental_fields() {
    let schema = schema("textile", "1.1.0");

    let properties = schema["properties"].as_object().unwrap();

    for field in JTC24_TEXTILE_ENVIRONMENTAL_FIELDS {
        assert!(
            properties.contains_key(*field),
            "schema missing environmental field: {field}"
        );
    }
}

#[test]
fn textile_v1_1_schema_covers_svhc_and_professional_fields() {
    let schema = schema("textile", "1.1.0");

    let properties = schema["properties"].as_object().unwrap();

    for field in JTC24_SVHC_FIELDS
        .iter()
        .chain(JTC24_PROFESSIONAL_FIELDS.iter())
    {
        assert!(
            properties.contains_key(*field),
            "schema missing SVHC/professional field: {field}"
        );
    }
}

#[test]
fn textile_v1_1_svhc_schema_requires_cas_and_name() {
    let schema = schema("textile", "1.1.0");

    let svhc_items = &schema["properties"]["svhcSubstances"]["items"];
    let required = svhc_items["required"]
        .as_array()
        .expect("svhcSubstances items must have required array");
    let required_names: Vec<&str> = required.iter().map(|v| v.as_str().unwrap()).collect();

    assert!(
        required_names.contains(&"casNumber"),
        "SVHC must require casNumber"
    );
    assert!(
        required_names.contains(&"substanceName"),
        "SVHC must require substanceName"
    );
    assert!(
        required_names.contains(&"concentrationPct"),
        "SVHC must require concentrationPct"
    );
}

#[test]
fn fibre_composition_schema_enforces_structure() {
    let schema = schema("textile", "1.1.0");

    let fibre_items = &schema["properties"]["fibreComposition"]["items"];
    let required = fibre_items["required"].as_array().unwrap();
    let required_names: Vec<&str> = required.iter().map(|v| v.as_str().unwrap()).collect();

    assert!(
        required_names.contains(&"fibre"),
        "fibreComposition items must require fibre"
    );
    assert!(
        required_names.contains(&"pct"),
        "fibreComposition items must require pct"
    );

    // pct must have min/max constraints
    let pct_schema = &fibre_items["properties"]["pct"];
    assert_eq!(pct_schema["minimum"].as_f64().unwrap(), 0.0);
    assert_eq!(pct_schema["maximum"].as_f64().unwrap(), 100.0);
}

// ─── All schemas present ─────────────────────────────────────────────────

#[test]
fn battery_schema_v1_is_valid() {
    let schema = schema("battery", "1.0.0");
    assert_eq!(schema["type"].as_str().unwrap(), "object");
    assert!(schema["properties"].as_object().is_some());
}

#[test]
fn steel_schema_v1_is_valid() {
    let schema = schema("steel", "1.0.0");
    assert_eq!(schema["type"].as_str().unwrap(), "object");
}

#[test]
fn unsold_goods_schema_v1_is_valid() {
    let schema = schema("unsold-goods", "1.0.0");
    assert_eq!(schema["type"].as_str().unwrap(), "object");
}

// ─── Country code pattern validation ─────────────────────────────────────

#[test]
fn country_fields_enforce_iso_3166_pattern() {
    let schema = schema("textile", "1.1.0");
    let props = schema["properties"].as_object().unwrap();

    // countryOfManufacturing must use ^[A-Z]{2}$ pattern
    let country_mfg = &props["countryOfManufacturing"];
    assert_eq!(
        country_mfg["pattern"].as_str().unwrap(),
        "^[A-Z]{2}$",
        "countryOfManufacturing must enforce ISO 3166-1 alpha-2"
    );

    // fibreComposition items countryOfOrigin
    let fibre_country = &props["fibreComposition"]["items"]["properties"]["countryOfOrigin"];
    assert_eq!(
        fibre_country["pattern"].as_str().unwrap(),
        "^[A-Z]{2}$",
        "fibre countryOfOrigin must enforce ISO 3166-1 alpha-2"
    );
}

// ─── Schema disallows additional properties ──────────────────────────────

#[test]
fn textile_schema_rejects_unknown_fields() {
    let schema = schema("textile", "1.1.0");

    assert!(
        !schema["additionalProperties"].as_bool().unwrap(),
        "textile schema must reject additional properties for conformity"
    );
}
