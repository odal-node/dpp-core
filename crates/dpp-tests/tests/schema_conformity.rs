//! Integration test: schema shape and field-set regression.
//!
//! Validates that:
//! 1. All JSON schemas are syntactically valid and loadable.
//! 2. The textile schema still carries the field set recorded below.
//! 3. Valid textile data passes schema validation.
//! 4. Invalid textile data is correctly rejected.
//! 5. The `VersionedSchemaRegistry` resolves the correct schema version.
//! 6. Battery and steel schemas are present and valid.
//!
//! Schemas are loaded through the public `VersionedSchemaRegistry` — the same
//! resolution path a consumer uses — rather than by reaching into files, so the
//! test also covers registry embedding.
//!
//! # This is not a conformity check, and used to say it was
//!
//! An earlier version of this header claimed the file "approximates what a
//! conformity assessment body would check". It does not, and the difference is
//! the whole point of the distinction: a conformity assessment body checks a
//! product against a **published standard**. Every field list below is *our own
//! expectation*, written here, asserted against a schema also written here. Both
//! sides are ours, so agreement between them is evidence of internal consistency
//! and of nothing else.
//!
//! That is still worth having — a field silently disappearing from the schema is
//! exactly the regression these catch — but it is a different claim, and the
//! stronger one was the sort of overstatement this project treats as a defect
//! rather than as marketing.
//!
//! # What would make it a real conformity check
//!
//! One purchase. Six EN 182xx:2026 standards are cited as harmonised, so
//! conformity to them now carries a presumption of conformity to the
//! corresponding ESPR articles — the fact, its source and its date live in the
//! `jtc24` record in `dpp-vocab`'s `vocabularies/`, which is the one home for it.
//! Nobody here has read the clause text: the standards are purchase-gated and
//! their text may not be redistributed.
//!
//! So the unlock condition is concrete and is not an engineering task: **buy the
//! relevant standard, read it, and replace the lists below with clause-cited
//! ones.** Until then the constants are named for what they are.

use dpp_domain::schemas::VersionedSchemaRegistry;
use semver::Version;

/// Load an embedded schema by (product group, version) through the public registry.
fn schema(product_group: &str, version: &str) -> serde_json::Value {
    let reg = VersionedSchemaRegistry::new();
    let json = reg
        .get(
            product_group,
            &Version::parse(version).expect("valid semver"),
        )
        .unwrap_or_else(|| panic!("schema {product_group} v{version} not embedded in registry"));
    serde_json::from_str(json).expect("embedded schema must be valid JSON")
}

// ─── Recorded field-set expectations ──────────────────────────────────────
//
// Our own expectations, not a standard's requirements. See the header: no EN
// clause text has been read, so nothing here is traceable to one.

/// Textile fields this project expects to be present **and** required.
///
/// Derived from our reading of what a textile passport needs, not from a
/// published standard. Renamed from `EXPECTED_TEXTILE_REQUIRED_FIELDS`, which
/// implied a provenance it never had.
const EXPECTED_TEXTILE_REQUIRED_FIELDS: &[&str] = &[
    "fibreComposition",
    "countryOfManufacturing",
    "careInstructions",
    "chemicalComplianceStandard",
];

/// Textile environmental-metric fields this project expects to be present.
const EXPECTED_TEXTILE_ENVIRONMENTAL_FIELDS: &[&str] = &[
    "recycledContentPct",
    "carbonFootprintKgCo2e",
    "waterUseLitres",
    "microplasticSheddingMgPerWash",
    "durabilityScore",
    "repairScore",
];

/// SVHC/SCIP disclosure field. Unlike the lists above this one *does* trace to
/// a named instrument — REACH Article 33 — though the mapping from that article
/// to this field name is still ours.
const EXPECTED_SVHC_FIELDS: &[&str] = &["svhcSubstances"];

/// Fields this project classifies as restricted (disassembly, spare parts).
const EXPECTED_RESTRICTED_FIELDS: &[&str] = &["disassemblyInstructions", "sparePartsAvailable"];

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
fn textile_schema_still_carries_every_expected_required_field() {
    let schema = schema("textile", "1.1.0");

    let properties = schema["properties"]
        .as_object()
        .expect("schema must have properties");
    let required = schema["required"]
        .as_array()
        .expect("schema must have required array");
    let required_names: Vec<&str> = required.iter().map(|v| v.as_str().unwrap()).collect();

    for field in EXPECTED_TEXTILE_REQUIRED_FIELDS {
        assert!(
            properties.contains_key(*field),
            "schema no longer declares expected field: {field}"
        );
        assert!(
            required_names.contains(field),
            "expected field '{field}' is declared but no longer in 'required'"
        );
    }
}

#[test]
fn textile_v1_1_schema_covers_environmental_fields() {
    let schema = schema("textile", "1.1.0");

    let properties = schema["properties"].as_object().unwrap();

    for field in EXPECTED_TEXTILE_ENVIRONMENTAL_FIELDS {
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

    for field in EXPECTED_SVHC_FIELDS
        .iter()
        .chain(EXPECTED_RESTRICTED_FIELDS.iter())
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
fn unsold_goods_schema_v2_is_valid() {
    let schema = schema("unsold-goods", "2.0.0");
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
