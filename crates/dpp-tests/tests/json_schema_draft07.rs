//! JSON Schema Draft-07: every shipped schema is a Draft-07 schema, and the
//! validator that enforces them passes the official suite for what they use.
//!
//! # Two claims, two kinds of evidence
//!
//! **The documents.** Each embedded product group schema is validated against the
//! Draft-07 meta-schema, vendored verbatim from json-schema.org. Before this,
//! `schema_conformity.rs` showed only that the schemas compile in the library we
//! use, which is circular: a schema that library tolerates and Draft-07 forbids
//! would have passed.
//!
//! **The validator.** The `jsonschema` crate we pin is run over the official
//! `JSON-Schema-Test-Suite` files for the keywords and formats the schemas use.
//! Every case passes and none is excluded.
//!
//! # What this cannot prove
//!
//! - **The meta-schema ignores keywords it does not know.** Draft-07 permits
//!   them, so a misspelt `minLenght` is a valid Draft-07 schema that constrains
//!   nothing. `the_metaschema_ignores_a_misspelt_keyword` asserts that, and
//!   `every_keyword_and_format_a_schema_uses_has_its_suite_file_vendored` is what
//!   closes it: any keyword that is not a Draft-07 one fails.
//! - **Nobody certifies a JSON Schema implementation.** The suite is the
//!   community's own, so a pass is a self-declared result and nothing more.
//! - **Only what the schemas use is run.** A keyword outside that set is outside
//!   the claim, and the census above turns the edge into a failure the day a
//!   schema crosses it.
//!
//! The vendored files, their source and their hashes are in
//! `fixtures/json-schema/NOTICE.md`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use dpp_domain::schemas::VersionedSchemaRegistry;
use jsonschema::Draft;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/json-schema");

/// The `$schema` every shipped schema declares.
const DRAFT07_ID: &str = "http://json-schema.org/draft-07/schema#";

fn read(relative: &str) -> String {
    fs::read_to_string(Path::new(FIXTURES).join(relative))
        .unwrap_or_else(|e| panic!("cannot read {relative}: {e}"))
}

/// Every file under `dir`, as a sorted `/`-separated path relative to it.
fn files_under(dir: &Path) -> Vec<String> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<String>) {
        let entries =
            fs::read_dir(dir).unwrap_or_else(|e| panic!("cannot scan {}: {e}", dir.display()));
        for entry in entries {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                walk(&path, root, out);
            } else {
                let relative = path.strip_prefix(root).expect("under the root");
                out.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

/// Every embedded schema, of every product group and version, as
/// (`"battery v2.8.0"`, parsed document).
fn shipped_schemas() -> Vec<(String, Value)> {
    let registry = VersionedSchemaRegistry::new();
    let mut out = Vec::new();
    for group in registry.product_groups() {
        for version in registry.versions_for(group) {
            let json = registry
                .get(group, version)
                .expect("the registry listed it");
            out.push((
                format!("{group} v{version}"),
                serde_json::from_str(json).expect("an embedded schema parses"),
            ));
        }
    }
    assert!(
        !out.is_empty(),
        "no schema was loaded, so nothing below would be checked"
    );
    out
}

fn metaschema() -> jsonschema::Validator {
    let document: Value =
        serde_json::from_str(&read("draft-07-schema.json")).expect("the meta-schema is JSON");
    jsonschema::draft7::new(&document).expect("the vendored meta-schema compiles")
}

// ─── The documents ────────────────────────────────────────────────────────

/// The constructor the registry uses, `validator_for`, reads `$schema`. So the
/// evidence below is about the draft that actually enforces a passport only if
/// that call resolves to Draft-07 for every shipped schema.
#[test]
fn every_shipped_schema_declares_draft07_and_is_compiled_as_draft07() {
    let mut wrong = Vec::new();
    for (name, schema) in shipped_schemas() {
        if schema.get("$schema").and_then(Value::as_str) != Some(DRAFT07_ID) {
            wrong.push(format!("{name}: `$schema` is not {DRAFT07_ID}"));
            continue;
        }
        match jsonschema::validator_for(&schema) {
            Ok(validator) if validator.draft() == Draft::Draft7 => {}
            Ok(validator) => wrong.push(format!("{name}: compiled as {:?}", validator.draft())),
            Err(e) => wrong.push(format!("{name}: does not compile: {e}")),
        }
    }
    assert!(
        wrong.is_empty(),
        "schemas not enforced as Draft-07:\n{}",
        wrong.join("\n")
    );
}

/// **The claim about the documents.**
#[test]
fn every_shipped_schema_is_valid_against_the_draft07_metaschema() {
    let meta = metaschema();
    let mut faults = Vec::new();
    for (name, schema) in shipped_schemas() {
        for error in meta.iter_errors(&schema) {
            faults.push(format!("{name} at `{}`: {error}", error.instance_path()));
        }
    }
    assert!(
        faults.is_empty(),
        "not valid Draft-07 schemas:\n{}",
        faults.join("\n")
    );
}

/// The check is only worth having if it rejects the shapes it is for.
#[test]
fn the_metaschema_check_rejects_what_it_is_for() {
    let meta = metaschema();
    for (why, schema) in [
        ("`type` names no JSON type", json!({"type": "strng"})),
        ("`required` is not an array", json!({"required": "name"})),
        ("`required` holds a non-string", json!({"required": [1]})),
        ("`minimum` is not a number", json!({"minimum": "1"})),
        ("`minLength` is negative", json!({"minLength": -1})),
        ("`enum` is not an array", json!({"enum": "a"})),
        ("`pattern` is not a string", json!({"pattern": 5})),
        (
            "`additionalProperties` is not a schema",
            json!({"additionalProperties": "no"}),
        ),
        (
            "a bad keyword nested in `properties` and `items`",
            json!({"properties": {"a": {"items": {"type": "nope"}}}}),
        ),
        (
            "a bad keyword inside `definitions`",
            json!({"definitions": {"d": {"pattern": 5}}}),
        ),
    ] {
        assert!(
            !meta.is_valid(&schema),
            "{why}: the meta-schema must reject {schema}"
        );
    }
    for schema in [
        json!({}),
        json!(true),
        json!({"type": ["string", "null"], "minLength": 1}),
    ] {
        assert!(
            meta.is_valid(&schema),
            "the meta-schema must accept {schema}"
        );
    }
}

/// The limit the module docs name, asserted rather than described.
#[test]
fn the_metaschema_ignores_a_misspelt_keyword() {
    assert!(
        metaschema().is_valid(&json!({"minLenght": 1})),
        "if the meta-schema now rejects an unknown keyword, the keyword census below is no \
         longer the only thing that catches a typo, and this test and its doc comment are stale"
    );
}

// ─── The validator ────────────────────────────────────────────────────────

/// Run one suite group, returning the id of every case the validator gets wrong,
/// or the group's own id if its schema does not compile.
fn check_group(file: &str, group: &Value) -> Vec<String> {
    let name = group["description"]
        .as_str()
        .expect("a group has a description");
    let Ok(validator) = jsonschema::draft7::new(&group["schema"]) else {
        return vec![format!("{file} :: {name}")];
    };
    group["tests"]
        .as_array()
        .expect("a group has tests")
        .iter()
        .filter(|case| {
            let expected = case["valid"]
                .as_bool()
                .expect("a test says whether it is valid");
            validator.is_valid(&case["data"]) != expected
        })
        .map(|case| {
            let test = case["description"]
                .as_str()
                .expect("a test has a description");
            format!("{file} :: {name} :: {test}")
        })
        .collect()
}

/// Every suite file vendored, relative to `test-suite/draft7/`.
fn vendored_suite_files() -> Vec<String> {
    let root = Path::new(FIXTURES).join("test-suite/draft7");
    files_under(&root)
}

/// **The claim about the validator.** Every vendored case passes, and nothing is
/// excluded. A `jsonschema` release that fails one fails this test, and whether to
/// hold the bump or to record an exclusion with its reason is decided then.
#[test]
fn every_vendored_suite_case_passes() {
    let files = vendored_suite_files();
    assert!(
        !files.is_empty(),
        "no suite file is vendored, so nothing would run"
    );

    let mut failed = Vec::new();
    let mut cases = 0usize;
    for file in &files {
        let groups: Vec<Value> = serde_json::from_str(&read(&format!("test-suite/draft7/{file}")))
            .unwrap_or_else(|e| panic!("{file} is not a suite file: {e}"));
        cases += groups
            .iter()
            .map(|g| g["tests"].as_array().map_or(0, Vec::len))
            .sum::<usize>();
        for group in &groups {
            failed.extend(check_group(file, group));
        }
    }
    assert!(cases > 0, "the suite files hold no case");
    assert!(
        failed.is_empty(),
        "{} of {cases} suite cases fail:\n{failed:#?}",
        failed.len()
    );
}

/// The harness must be able to fail: a wrong verdict, and a schema that does not
/// compile, are both reported.
#[test]
fn the_suite_harness_reports_what_it_is_for() {
    let wrong_verdict = json!({
        "description": "g",
        "schema": {"type": "string"},
        "tests": [
            {"description": "right", "data": "a", "valid": true},
            {"description": "wrong", "data": 1, "valid": true}
        ]
    });
    assert_eq!(
        check_group("f.json", &wrong_verdict),
        ["f.json :: g :: wrong"]
    );

    let unresolvable = json!({
        "description": "g",
        "schema": {"$ref": "#/definitions/missing"},
        "tests": [{"description": "t", "data": 1, "valid": true}]
    });
    assert_eq!(check_group("f.json", &unresolvable), ["f.json :: g"]);
}

// ─── What the schemas use ─────────────────────────────────────────────────

/// Keywords that only describe or identify a schema and constrain no instance.
const ANNOTATIONS: &[&str] = &[
    "$schema",
    "$id",
    "$comment",
    "title",
    "description",
    "examples",
    "readOnly",
];

/// Every other Draft-07 keyword, with the suite file that tests it. A keyword
/// absent from both lists is not a Draft-07 keyword.
const KEYWORD_FILES: &[(&str, &str)] = &[
    ("$ref", "ref.json"),
    ("additionalItems", "additionalItems.json"),
    ("additionalProperties", "additionalProperties.json"),
    ("allOf", "allOf.json"),
    ("anyOf", "anyOf.json"),
    ("const", "const.json"),
    ("contains", "contains.json"),
    ("contentEncoding", "optional/content.json"),
    ("contentMediaType", "optional/content.json"),
    ("default", "default.json"),
    ("definitions", "definitions.json"),
    ("dependencies", "dependencies.json"),
    ("else", "if-then-else.json"),
    ("enum", "enum.json"),
    ("exclusiveMaximum", "exclusiveMaximum.json"),
    ("exclusiveMinimum", "exclusiveMinimum.json"),
    ("format", "format.json"),
    ("if", "if-then-else.json"),
    ("items", "items.json"),
    ("maxItems", "maxItems.json"),
    ("maxLength", "maxLength.json"),
    ("maxProperties", "maxProperties.json"),
    ("maximum", "maximum.json"),
    ("minItems", "minItems.json"),
    ("minLength", "minLength.json"),
    ("minProperties", "minProperties.json"),
    ("minimum", "minimum.json"),
    ("multipleOf", "multipleOf.json"),
    ("not", "not.json"),
    ("oneOf", "oneOf.json"),
    ("pattern", "pattern.json"),
    ("patternProperties", "patternProperties.json"),
    ("properties", "properties.json"),
    ("propertyNames", "propertyNames.json"),
    ("required", "required.json"),
    ("then", "if-then-else.json"),
    ("type", "type.json"),
    ("uniqueItems", "uniqueItems.json"),
];

/// Record every keyword and `format` value used by the schema nodes under
/// `node`. Property names, `enum` and `const` values are data, not keywords, so
/// only schema positions are entered.
fn collect_usage(node: &Value, keywords: &mut BTreeSet<String>, formats: &mut BTreeSet<String>) {
    let Value::Object(map) = node else { return };
    keywords.extend(map.keys().cloned());
    if let Some(format) = map.get("format").and_then(Value::as_str) {
        formats.insert(format.to_owned());
    }
    for key in [
        "properties",
        "patternProperties",
        "definitions",
        "dependencies",
    ] {
        if let Some(Value::Object(children)) = map.get(key) {
            children
                .values()
                .for_each(|child| collect_usage(child, keywords, formats));
        }
    }
    for key in [
        "items",
        "additionalItems",
        "additionalProperties",
        "contains",
        "propertyNames",
        "not",
        "if",
        "then",
        "else",
    ] {
        match map.get(key) {
            Some(Value::Array(children)) => {
                children
                    .iter()
                    .for_each(|child| collect_usage(child, keywords, formats));
            }
            Some(child) => collect_usage(child, keywords, formats),
            None => {}
        }
    }
    for key in ["allOf", "anyOf", "oneOf"] {
        if let Some(Value::Array(children)) = map.get(key) {
            children
                .iter()
                .for_each(|child| collect_usage(child, keywords, formats));
        }
    }
}

/// What the claim rests on stays what the schemas use. The suite files are only
/// the ones for the keywords and formats in use, so a schema that starts using
/// another would otherwise sit outside the evidence without anything saying so.
#[test]
fn every_keyword_and_format_a_schema_uses_has_its_suite_file_vendored() {
    let mut keywords = BTreeSet::new();
    let mut formats = BTreeSet::new();
    for (_, schema) in shipped_schemas() {
        collect_usage(&schema, &mut keywords, &mut formats);
    }
    assert!(
        keywords.contains("properties"),
        "the census found no keyword at all"
    );

    let vendored: BTreeSet<String> = vendored_suite_files().into_iter().collect();
    let mut problems = Vec::new();
    for keyword in &keywords {
        if keyword.starts_with("x-") || ANNOTATIONS.contains(&keyword.as_str()) {
            continue;
        }
        match KEYWORD_FILES.iter().find(|(k, _)| k == keyword) {
            None => problems.push(format!(
                "`{keyword}` is not a Draft-07 keyword. The meta-schema ignores it, so it \
                 constrains nothing"
            )),
            Some((_, file)) if !vendored.contains(*file) => problems.push(format!(
                "`{keyword}` is used, but tests/draft7/{file} is not vendored: take it from the \
                 commit in fixtures/json-schema/NOTICE.md and add it to that table"
            )),
            Some(_) => {}
        }
    }
    for format in &formats {
        let file = format!("optional/format/{format}.json");
        if !vendored.contains(&file) {
            problems.push(format!(
                "format `{format}` is used, but tests/draft7/{file} is not vendored: take it \
                 from the commit in fixtures/json-schema/NOTICE.md and add it to that table"
            ));
        }
    }
    assert!(
        problems.is_empty(),
        "the schemas outgrew their evidence:\n{}",
        problems.join("\n")
    );
}

/// The workspace builds `jsonschema` without remote retrieval, so a `$ref` that
/// leaves its document could not be resolved, and the suite's remote-reference
/// file is not vendored. That is only sound while no shipped schema refers to
/// another document.
#[test]
fn every_ref_in_a_shipped_schema_is_local() {
    fn refs(node: &Value, out: &mut Vec<String>) {
        match node {
            Value::Object(map) => {
                for (key, value) in map {
                    match (key.as_str(), value) {
                        ("$ref", Value::String(target)) => out.push(target.clone()),
                        _ => refs(value, out),
                    }
                }
            }
            Value::Array(items) => items.iter().for_each(|item| refs(item, out)),
            _ => {}
        }
    }
    let mut remote = Vec::new();
    let mut seen = 0usize;
    for (name, schema) in shipped_schemas() {
        let mut found = Vec::new();
        refs(&schema, &mut found);
        seen += found.len();
        remote.extend(
            found
                .into_iter()
                .filter(|t| !t.starts_with("#/"))
                .map(|t| format!("{name}: {t}")),
        );
    }
    assert!(seen > 0, "no `$ref` was found, so nothing was checked");
    assert!(
        remote.is_empty(),
        "a `$ref` leaves its document:\n{}",
        remote.join("\n")
    );
}

// ─── The vendored bytes ───────────────────────────────────────────────────

/// The files are evidence only while they are upstream's bytes. A suite file
/// edited until it passes would otherwise pass.
#[test]
fn the_vendored_files_are_the_bytes_recorded_here() {
    let notice = read("NOTICE.md");
    let unquote = |cell: &str| {
        cell.strip_prefix('`')
            .and_then(|c| c.strip_suffix('`'))
            .map(str::to_owned)
    };

    let mut recorded: BTreeMap<String, (u64, String)> = BTreeMap::new();
    for line in notice.lines() {
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        if cells.len() != 5 {
            continue;
        }
        if let (Some(path), Ok(size), Some(hash)) = (
            unquote(cells[1]),
            cells[2].parse::<u64>(),
            unquote(cells[3]),
        ) {
            recorded.insert(path, (size, hash));
        }
    }
    assert!(
        !recorded.is_empty(),
        "NOTICE.md records no file, so nothing would be checked"
    );

    let mut faults = Vec::new();
    for (path, (size, hash)) in &recorded {
        match fs::read(Path::new(FIXTURES).join(path)) {
            Err(e) => faults.push(format!("{path}: cannot be read: {e}")),
            Ok(bytes) => {
                if bytes.len() as u64 != *size {
                    faults.push(format!(
                        "{path}: {} bytes, NOTICE.md records {size}",
                        bytes.len()
                    ));
                }
                let actual = hex::encode(Sha256::digest(&bytes));
                if actual != *hash {
                    faults.push(format!(
                        "{path}: SHA-256 {actual}, NOTICE.md records {hash}"
                    ));
                }
            }
        }
    }
    for path in files_under(Path::new(FIXTURES)) {
        if path != "NOTICE.md" && !recorded.contains_key(&path) {
            faults.push(format!(
                "{path}: is vendored but NOTICE.md does not record it"
            ));
        }
    }
    assert!(
        faults.is_empty(),
        "vendored files differ from NOTICE.md:\n{}",
        faults.join("\n")
    );
}
