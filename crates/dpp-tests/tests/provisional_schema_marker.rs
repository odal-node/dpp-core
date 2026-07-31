//! Every schema for a sector whose act is not in force is marked a draft, and
//! every schema for one that is in force is not.
//!
//! The catalog manifest is the single home for a sector's regulatory status.
//! What this gate adds is that the *consequence* of that status cannot be
//! forgotten: a schema is a machine-readable promise about a data model, and a
//! provisional sector's schema is our reading of an instrument nobody has
//! ratified. Someone reading the file has to be able to see that without
//! knowing to go and check a manifest.
//!
//! **This is the go sign.** Setting `"status": "in_force"` in the sector
//! manifest makes this test fail until the draft marker is removed — so
//! promoting a sector is a deliberate two-part edit rather than a silent flip,
//! and demoting one is caught the same way.

use std::fs;
use std::path::Path;

use dpp_domain::SectorCatalog;

const MARKER_PREFIX: &str = "DRAFT — NOT IN FORCE";

fn schema_dir(key: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../dpp-domain/schemas")
        .join(key)
}

fn schema_files(key: &str) -> Vec<std::path::PathBuf> {
    let dir = schema_dir(key);
    let mut files: Vec<_> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("sector '{key}' has no schema directory at {dir:?}: {e}"))
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "sector '{key}' has no schema files");
    files
}

fn is_marked_draft(path: &Path) -> bool {
    let raw = fs::read_to_string(path).expect("read schema");
    let value: serde_json::Value = serde_json::from_str(&raw).expect("schema is valid JSON");
    value
        .get("$comment")
        .and_then(|c| c.as_str())
        .is_some_and(|c| c.starts_with(MARKER_PREFIX))
}

#[test]
fn schema_draft_marker_matches_catalog_status() {
    let catalog = SectorCatalog::new();
    let mut checked = 0;

    for descriptor in catalog.all() {
        let key = descriptor.key.as_str();
        let in_force = catalog.is_in_force(key);

        for path in schema_files(key) {
            let marked = is_marked_draft(&path);
            let name = path.file_name().unwrap_or_default().to_string_lossy();

            if in_force {
                assert!(
                    !marked,
                    "sector '{key}' is in force but its schema {name} is still marked a draft. \
                     If this sector was just promoted, remove the `$comment` draft marker — \
                     that removal is the second half of the go sign."
                );
            } else {
                assert!(
                    marked,
                    "sector '{key}' is not in force but its schema {name} carries no draft \
                     marker. Add a `$comment` beginning \"{MARKER_PREFIX}\", so a reader of the \
                     schema alone can tell it describes an instrument that has not been ratified."
                );
            }
            checked += 1;
        }
    }

    assert!(
        checked > 0,
        "no schemas were checked — the gate is not running"
    );
}

/// The marker is not merely present; it says what it needs to say. A reader who
/// finds it should learn that validating against the schema is not evidence of
/// compliance, and where the go sign lives.
#[test]
fn the_draft_marker_states_its_consequences() {
    let catalog = SectorCatalog::new();

    for descriptor in catalog.all() {
        let key = descriptor.key.as_str();
        if catalog.is_in_force(key) {
            continue;
        }
        for path in schema_files(key) {
            let raw = fs::read_to_string(&path).expect("read schema");
            let value: serde_json::Value = serde_json::from_str(&raw).expect("valid JSON");
            let comment = value
                .get("$comment")
                .and_then(|c| c.as_str())
                .unwrap_or_default();

            for required in ["not evidence of compliance", "in_force"] {
                assert!(
                    comment.contains(required),
                    "sector '{key}': the draft marker must mention '{required}' — a marker that \
                     does not say what it costs the reader is decoration"
                );
            }
        }
    }
}
