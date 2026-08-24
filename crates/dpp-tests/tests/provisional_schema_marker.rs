//! Every schema says, in the file itself, what it is worth.
//!
//! A schema is a machine-readable promise about a data model. Two things can be
//! wrong with that promise and neither is visible from the file:
//!
//! - **no act binds the product group yet**, so the schema is our reading of an
//!   instrument nobody has ratified; or
//! - **an act binds it but none asks for a passport**, so the schema describes
//!   data we model rather than a passport the law requires.
//!
//! The second is not hypothetical. `electronics` shipped as in force with a
//! passport date taken from an *ecodesign* application date, when Regs (EU)
//! 2023/1670 and 2023/1669 impose no passport at all and route through EPREL;
//! `unsold-goods` shipped with the ESPR Art. 25 destruction-ban date, from two
//! articles containing no passport either. Both schemas carried no marker,
//! because the only question being asked was "is it in force" — which for both
//! of them is *yes*.
//!
//! So there are two markers and three states, and this gate holds the file and
//! the catalogs to the same answer.
//!
//! **This is the go sign.** Flipping a binding to `in_force` in the instrument
//! manifest makes this test fail until the marker is removed — so promoting a
//! product group is a deliberate two-part edit rather than a silent flip, and
//! demoting one is caught the same way.

use std::fs;
use std::path::Path;

use dpp_domain::{InstrumentCatalog, ProductGroupCatalog};

/// No act binding this product group is in force.
const DRAFT: &str = "DRAFT — NOT IN FORCE";
/// An act binds it, and no act requires a passport for it.
const NO_PASSPORT: &str = "NO PASSPORT OBLIGATION";

/// What a schema's `$comment` must open with, given what the catalogs say.
/// `None` means the schema describes a passport some in-force act actually
/// requires, and needs no warning.
fn expected_marker(key: &str, instruments: &InstrumentCatalog) -> Option<&'static str> {
    if instruments.determinable_for(key).is_empty() {
        Some(DRAFT)
    } else if !instruments.passport_required_for(key) {
        Some(NO_PASSPORT)
    } else {
        None
    }
}

fn schema_dir(key: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../dpp-domain/schemas")
        .join(key)
}

fn schema_files(key: &str) -> Vec<std::path::PathBuf> {
    let dir = schema_dir(key);
    let mut files: Vec<_> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("product group '{key}' has no schema directory at {dir:?}: {e}"))
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    assert!(
        !files.is_empty(),
        "product group '{key}' has no schema files"
    );
    files
}

fn comment(path: &Path) -> String {
    let raw = fs::read_to_string(path).expect("read schema");
    let value: serde_json::Value = serde_json::from_str(&raw).expect("schema is valid JSON");
    value
        .get("$comment")
        .and_then(|c| c.as_str())
        .unwrap_or_default()
        .to_owned()
}

#[test]
fn schema_marker_matches_what_the_acts_say() {
    let catalog = ProductGroupCatalog::new();
    let instruments = InstrumentCatalog::new();
    let mut checked = 0;

    for descriptor in catalog.all() {
        let key = descriptor.key.as_str();
        let expected = expected_marker(key, &instruments);

        for path in schema_files(key) {
            let found = comment(&path);
            let name = path.file_name().unwrap_or_default().to_string_lossy();

            match expected {
                Some(marker) => assert!(
                    found.starts_with(marker),
                    "product group '{key}': schema {name} must open its `$comment` with \
                     \"{marker}\", so a reader of the schema alone learns what it is worth. \
                     Found: {found:.80}"
                ),
                None => assert!(
                    !found.starts_with(DRAFT) && !found.starts_with(NO_PASSPORT),
                    "product group '{key}' has an in-force act requiring a passport, but its \
                     schema {name} still carries a warning marker. If it was just promoted, \
                     removing the marker is the second half of the go sign."
                ),
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
/// compliance, and where the thing that would change that lives.
#[test]
fn the_marker_states_its_consequences() {
    let catalog = ProductGroupCatalog::new();
    let instruments = InstrumentCatalog::new();

    for descriptor in catalog.all() {
        let key = descriptor.key.as_str();
        let Some(marker) = expected_marker(key, &instruments) else {
            continue;
        };
        // A draft points at the binding status that would promote it; a schema
        // with no passport behind it points at the obligation instead, because
        // no status change can make a passport out of an act that requires none.
        let required = if marker == DRAFT {
            ["not evidence of compliance", "in_force"]
        } else {
            ["not evidence of compliance", "passport"]
        };

        for path in schema_files(key) {
            let found = comment(&path);
            for phrase in required {
                assert!(
                    found.contains(phrase),
                    "product group '{key}': the marker must mention '{phrase}' — a marker that \
                     does not say what it costs the reader is decoration"
                );
            }
        }
    }
}
