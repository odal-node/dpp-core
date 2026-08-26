//! Frozen product group-data documents, one per schema version this crate claims to
//! support, asserted to still be readable.
//!
//! # Why this exists
//!
//! `Passport` and every `ProductGroupData` variant are the literal on-disk shape of
//! every passport a node has ever stored. A non-additive change to one of them
//! — a field gaining a requirement, a wire key being renamed — does not break a
//! consumer at compile time. It makes every already-written document of that
//! shape undeserialisable the moment a node upgrades its `dpp-domain` pin: a
//! runtime failure against data, discovered per request, with no compile-time
//! signal anywhere.
//!
//! That has happened. `TextileData.gtin` became required and
//! `countryOfManufacturing` was renamed to `countryOfOrigin`, and downstream
//! that took out reads for 244 of 276 passports the instant the node upgraded.
//!
//! `ProductGroupCatalog` declares which versions each product group supports. This crate is
//! the one in a position to check that its own types still parse everything it
//! claims — rather than leaving each consumer to discover otherwise.
//!
//! # What a fixture is
//!
//! One minimal document per `(product group, version)`, holding exactly the fields the
//! schema makes required. It is **frozen**: written once and never regenerated,
//! because a fixture regenerated from the current schema would agree with the
//! current schema by construction and could not catch anything.
//!
//! Fixtures are stored schema-shaped, without the `product_group` discriminant. Every
//! schema sets `additionalProperties: false` and none declares `product_group`, so a
//! document carrying the tag could not validate; the tag is injected here for
//! the deserialisation half.
//!
//! # Adding a schema version
//!
//! `just freeze-schema-fixtures` writes a fixture for any declared version that
//! lacks one. It never overwrites an existing file. If a generated fixture does
//! not validate — a pattern-constrained field it cannot invent a value for — it
//! says so and writes nothing, and the fixture is authored by hand.

use dpp_domain::catalog::ProductGroupCatalog;
use dpp_domain::passport::Passport;
use dpp_domain::product_group::ProductGroup;
use dpp_domain::schemas::VersionedSchemaRegistry;
use dpp_domain::schemas::lens::LensRegistry;
use serde_json::{Map, Value};

/// Where frozen fixtures live, relative to this crate's root.
const FIXTURE_DIR: &str = "tests/fixtures/schema-compat";

fn fixture_path(product_group: &str, version: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(FIXTURE_DIR)
        .join(product_group)
        .join(format!("v{version}.json"))
}

/// Every `(product group, version)` the registry serves, as owned strings.
fn declared_versions() -> Vec<(String, String)> {
    let registry = VersionedSchemaRegistry::new();
    let mut out: Vec<(String, String)> = registry
        .list()
        .into_iter()
        .map(|(product_group, version)| (product_group.to_owned(), version.to_string()))
        .collect();
    out.sort();
    out
}

fn load_fixture(product_group: &str, version: &str) -> Option<Value> {
    let raw = std::fs::read_to_string(fixture_path(product_group, version)).ok()?;
    Some(serde_json::from_str(&raw).expect("a frozen fixture must be valid JSON"))
}

// ── The three properties ─────────────────────────────────────────────────────

/// A declared version with no fixture is an unguarded version.
///
/// This is what makes the gate a tripwire rather than a suite: adding a schema
/// version fails here until a fixture is frozen alongside it.
#[test]
fn every_declared_schema_version_has_a_frozen_fixture() {
    let missing: Vec<String> = declared_versions()
        .into_iter()
        .filter(|(product_group, version)| load_fixture(product_group, version).is_none())
        .map(|(product_group, version)| format!("{product_group} v{version}"))
        .collect();

    assert!(
        missing.is_empty(),
        "these schema versions are declared but have no frozen fixture, so nothing \
         checks that documents written under them are still readable: {missing:?}\n\
         Run `just freeze-schema-fixtures` to write the missing ones."
    );
}

/// A fixture must be a legal document under the schema it was frozen for.
///
/// Guards the fixtures themselves: one that never validated would pass the
/// deserialisation check below while proving nothing about real data.
#[test]
fn every_frozen_fixture_validates_against_its_own_schema() {
    let registry = VersionedSchemaRegistry::new();

    for (product_group, version) in declared_versions() {
        let Some(fixture) = load_fixture(&product_group, &version) else {
            continue; // reported by the fixture-presence test
        };
        let result = registry.validate_strict(&product_group, &version, &fixture);
        assert!(
            result.is_ok(),
            "the frozen fixture for {product_group} v{version} does not validate against \
             the schema it was frozen for: {:?}",
            result.err()
        );
    }
}

/// **The tripwire.** Today's Rust types must still read every frozen document.
///
/// This is the half that catches a non-additive change. A field gaining a
/// requirement, or a wire key being renamed without an alias, fails here — at
/// the commit that introduces it, rather than in a node's logs after an upgrade.
/// A stored passport document wrapping `product_group_data`, as a node would hold it.
///
/// Built as raw JSON rather than from a Rust value on purpose: the thing under
/// test is whether a *document* still reads, and constructing it from today's
/// types would bake today's shape into the fixture.
fn stored_passport(product_group: &str, version: &str, mut product_group_data: Value) -> Value {
    // `ProductGroupData` is tagged by an inner `product_group` key, which the schemas do not
    // declare (they are all `additionalProperties: false`), so the fixture is
    // stored without it and it is added here.
    product_group_data
        .as_object_mut()
        .expect("a fixture is a JSON object")
        .insert(
            "productGroup".to_owned(),
            Value::String(
                ProductGroup::from_wire_tag(product_group)
                    .wire_str()
                    .to_owned(),
            ),
        );
    serde_json::json!({
        "id": "01926b7e-0000-7000-8000-000000000000",
        "batchId": null,
        "productName": "Compatibility fixture",
        "productGroup": ProductGroup::from_wire_tag(product_group).wire_str(),
        "manufacturer": { "name": "Example GmbH", "address": "Example Str. 1, Berlin" },
        "materials": [],
        "co2ePerUnit": null,
        "repairabilityScore": null,
        "productGroupData": product_group_data,
        "status": "draft",
        "qrCodeUrl": null,
        "jwsSignature": null,
        "createdAt": "2024-01-01T00:00:00Z",
        "updatedAt": "2024-01-01T00:00:00Z",
        "publishedAt": null,
        "schemaVersion": version,
        "retentionLocked": false,
    })
}

/// **The tripwire.** Every frozen document must still be readable through the
/// path a node actually reads stored documents by.
///
/// That path is `Passport::from_stored`: deserialise directly, and on failure
/// upcast from the document's recorded `schemaVersion` to the catalog's current
/// version through the lens registry. Testing the bare `ProductGroupData` instead
/// would assert a contract that does not exist — old documents are *meant* to
/// arrive through a lens, and a product group whose rename ships one is not broken.
///
/// So this fails for exactly one reason: a stored document that no longer reads
/// and has no lens carrying it forward.
#[test]
fn every_frozen_document_still_reads_through_from_stored() {
    let lenses = LensRegistry::new();
    let catalog = ProductGroupCatalog::new();

    // Collected rather than asserted per version: the interesting question is
    // how much stored data a change orphans, and stopping at the first hides
    // that one rename swept several product groups at once.
    let mut broken: Vec<String> = Vec::new();

    for (product_group, version) in declared_versions() {
        let Some(fixture) = load_fixture(&product_group, &version) else {
            continue; // reported by the fixture-presence test
        };
        let doc = stored_passport(&product_group, &version, fixture);
        if let Err(e) = Passport::from_stored(doc, &lenses, &catalog) {
            broken.push(format!("{product_group} v{version}: {e}"));
        }
    }

    // Battery before v2.5.0 is a deliberate, documented refusal rather than an
    // oversight: v2.5.0 made `batteryType` required because Annex VI Part A
    // point 2 makes the battery category mandatory public content, and the
    // v2.4.0 → v2.5.0 lens refuses rather than inventing a category for a record
    // that predates the mandate. There is no correct value to supply, so the
    // honest outcome is a refusal.
    //
    // Listed exactly, so this fails if the set grows *or* shrinks. A new orphan
    // is a regression; a fixed one that stays listed is a stale exemption.
    let expected_refusals = [
        "battery v1.0.0",
        "battery v2.0.0",
        "battery v2.1.0",
        "battery v2.2.0",
        "battery v2.3.0",
        "battery v2.4.0",
    ];
    let unexpected: Vec<&String> = broken
        .iter()
        .filter(|b| !expected_refusals.iter().any(|e| b.starts_with(e)))
        .collect();
    assert!(
        unexpected.is_empty(),
        "documents written under these schema versions can no longer be read, and \
         no lens carries them forward. Every passport stored under one of them is \
         undeserialisable — a runtime failure against data, per request, with no \
         compile-time signal. Make the change additive (`Option<T>` plus \
         `#[serde(default)]`, or a rename that still accepts the old key), or ship \
         a lens.\n{unexpected:#?}"
    );
    let missing: Vec<&str> = expected_refusals
        .iter()
        .filter(|e| !broken.iter().any(|b| b.starts_with(*e)))
        .copied()
        .collect();
    assert!(
        missing.is_empty(),
        "these versions are listed as deliberate refusals but now read \
         successfully. If that is intended, remove them from the list so it keeps \
         describing reality: {missing:?}"
    );
}

// ── Freezing ─────────────────────────────────────────────────────────────────

/// Write a fixture for any declared version that lacks one.
///
/// Gated behind an environment variable, following `gs1_oracle_corpus`: it
/// writes files, which a test run must not do by default. It **never**
/// overwrites — a frozen fixture that could be regenerated is not frozen.
///
/// ```text
/// FREEZE_SCHEMA_FIXTURES=1 cargo test -p dpp-domain --test schema_compat
/// ```
#[test]
fn freeze_missing_fixtures() {
    if std::env::var("FREEZE_SCHEMA_FIXTURES").is_err() {
        return;
    }

    let registry = VersionedSchemaRegistry::new();
    let mut wrote = 0usize;
    let mut refused: Vec<String> = Vec::new();

    for (product_group, version) in declared_versions() {
        let path = fixture_path(&product_group, &version);
        if path.exists() {
            continue;
        }
        let parsed: semver::Version = version.parse().expect("registry versions are semver");
        let schema: Value = serde_json::from_str(
            registry
                .get(&product_group, &parsed)
                .expect("the registry just listed this version"),
        )
        .expect("an embedded schema is valid JSON");

        let candidate = minimal_document(&schema, &schema);

        // Refuse to write a fixture that does not validate. A fixture that was
        // never legal proves nothing, and a silently wrong one is worse than a
        // missing one that fails loudly.
        if let Err(e) = registry.validate_strict(&product_group, &version, &candidate) {
            refused.push(format!("{product_group} v{version}: {e:?}"));
            continue;
        }

        std::fs::create_dir_all(path.parent().expect("fixture path has a parent"))
            .expect("cannot create fixture directory");
        let mut text = serde_json::to_string_pretty(&candidate).expect("fixture serialises");
        text.push('\n');
        std::fs::write(&path, text).expect("cannot write fixture");
        wrote += 1;
    }

    assert!(
        refused.is_empty(),
        "could not generate a valid fixture for these versions; author them by hand \
         under {FIXTURE_DIR}: {refused:#?}"
    );
    eprintln!("froze {wrote} fixture(s)");
}

/// Build the smallest document the schema accepts: every required property,
/// nothing optional.
///
/// Deliberately not a general JSON Schema instance generator. It covers the
/// constructs these schemas use and refuses (by producing something that fails
/// validation, which `freeze_missing_fixtures` then reports) rather than
/// guessing at anything else.
fn minimal_document(schema: &Value, root: &Value) -> Value {
    let mut out = Map::new();
    let properties = schema.get("properties").and_then(Value::as_object);
    let required = schema
        .get("required")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();

    for name in required.iter().filter_map(Value::as_str) {
        let Some(prop) = properties.and_then(|p| p.get(name)) else {
            continue;
        };
        out.insert(name.to_owned(), minimal_value(prop, root));
    }
    Value::Object(out)
}

/// A value satisfying one property schema.
fn minimal_value(prop: &Value, root: &Value) -> Value {
    // `$ref` into the schema's own `$defs`/`definitions`.
    if let Some(reference) = prop.get("$ref").and_then(Value::as_str)
        && let Some(target) = resolve_ref(reference, root)
    {
        return minimal_value(&target, root);
    }
    // A closed set answers the question for us, and is the only source that can
    // be trusted for a value with no other constraint.
    if let Some(first) = prop
        .get("enum")
        .and_then(Value::as_array)
        .and_then(|e| e.first())
    {
        return first.clone();
    }
    if let Some(constant) = prop.get("const") {
        return constant.clone();
    }
    for keyword in ["allOf", "anyOf", "oneOf"] {
        if let Some(first) = prop
            .get(keyword)
            .and_then(Value::as_array)
            .and_then(|v| v.first())
        {
            return minimal_value(first, root);
        }
    }

    match prop.get("type").and_then(Value::as_str) {
        Some("string") => Value::String(minimal_string(prop)),
        Some("integer") => Value::from(
            prop.get("minimum")
                .and_then(Value::as_i64)
                .unwrap_or(1)
                .max(0),
        ),
        Some("number") => Value::from(prop.get("minimum").and_then(Value::as_f64).unwrap_or(1.0)),
        Some("boolean") => Value::Bool(false),
        Some("array") => {
            let min = prop.get("minItems").and_then(Value::as_u64).unwrap_or(0);
            let items = prop
                .get("items")
                .map_or_else(|| Value::Null, |items| minimal_value(items, root));
            Value::Array((0..min).map(|_| items.clone()).collect())
        }
        Some("object") => minimal_document(prop, root),
        // No `type`: an object with `properties` is still an object.
        _ if prop.get("properties").is_some() => minimal_document(prop, root),
        _ => Value::Null,
    }
}

/// Representative values for the patterns these schemas actually use.
///
/// Deliberately a small explicit table rather than a regex solver. Each entry
/// is a value a reviewer can check against the pattern by eye, and an unknown
/// pattern is refused rather than guessed at — a fixture that happens to pass
/// validation but means nothing is worse than one that is missing.
const PATTERN_VALUES: &[(&str, &str)] = &[
    // A GTIN-14. Check-digit valid, not merely fourteen digits: the schema only
    // asks for the digits, but `Gtin::parse` verifies the check digit, and the
    // fixture has to survive both halves of this gate.
    (r"^[0-9]{14}$", "09506000134352"),
    // ISO 3166-1 alpha-2.
    (r"^[A-Z]{2}$", "DE"),
];

/// A string satisfying whatever the property constrains.
///
/// `pattern` is the one case that cannot be satisfied generically. Rather than
/// invent a value that happens to pass, this returns a marker that will fail
/// validation, so the version is reported and authored by hand.
fn minimal_string(prop: &Value) -> String {
    match prop.get("format").and_then(Value::as_str) {
        Some("date") => return "2024-01-01".to_owned(),
        Some("date-time") => return "2024-01-01T00:00:00Z".to_owned(),
        Some("uri") => return "https://example.com/".to_owned(),
        Some("email") => return "someone@example.com".to_owned(),
        _ => {}
    }
    if let Some(pattern) = prop.get("pattern").and_then(Value::as_str) {
        return PATTERN_VALUES
            .iter()
            .find(|(p, _)| *p == pattern)
            .map_or_else(
                || format!("UNKNOWN-PATTERN-AUTHOR-BY-HAND<{pattern}>"),
                |(_, v)| (*v).to_owned(),
            );
    }
    let min = prop.get("minLength").and_then(Value::as_u64).unwrap_or(1);
    let base = "x".repeat(usize::try_from(min.max(1)).unwrap_or(1));
    match prop.get("maxLength").and_then(Value::as_u64) {
        Some(max) if (base.len() as u64) > max => "x".repeat(usize::try_from(max).unwrap_or(1)),
        _ => base,
    }
}

/// Resolve a local `#/$defs/Name` or `#/definitions/Name` reference.
fn resolve_ref(reference: &str, root: &Value) -> Option<Value> {
    let path = reference.strip_prefix("#/")?;
    let mut node = root;
    for segment in path.split('/') {
        node = node.get(segment)?;
    }
    Some(node.clone())
}
