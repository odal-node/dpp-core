//! Tripwire: a payload's GTIN keeps the type that validates it.
//!
//! GTIN validity across every product group rests on one declaration per
//! payload — `gtin: Gtin` — plus `Gtin`'s hand-written `Deserialize`, which
//! calls `Gtin::parse`. Together those refuse a bad GS1 check digit while a
//! document is being deserialised, for every product group at once, before any
//! caller sees the value.
//!
//! Nothing asserted that the arrangement holds, and it fails silently. Change
//! one payload's field to `gtin: String` and validation for that product group
//! disappears without anything going red: the crate compiles, the schema's
//! `^[0-9]{14}$` pattern still passes (it checks shape, not the check digit),
//! and `ProductGroupPayload::gtin` still returns `Some(self.gtin.as_str())`
//! because `String` has `as_str()` too. Every fixture in this workspace builds
//! its GTIN through `Gtin::parse`, so all of them are valid by construction and
//! none of them could catch it.
//!
//! The invariant has two halves and this file pins both:
//!
//! 1. **The type refuses an invalid GTIN through serde** — behavioural, tested
//!    directly on `Gtin`, so it needs no payload fixture.
//! 2. **Every payload that declares a GTIN uses that type** — structural, read
//!    from the source, so a new product group cannot be added without one.
//!
//! Neither half is sufficient alone: the first says the lock works, the second
//! says every door has it fitted.

use std::fs;
use std::path::{Path, PathBuf};

use dpp_domain::{Gtin, ProductGroupData};

/// A GTIN whose 14-digit shape is valid and whose GS1 mod-10 check digit is not.
///
/// Shape-valid on purpose. The schema pattern accepts this string, so the schema
/// cannot be what rejects it — only the type can, which is the whole point.
const BAD_CHECK_DIGIT: &str = "09506000134353";

/// The same trade item number with its correct check digit.
const GOOD: &str = "09506000134352";

// ---------------------------------------------------------------------------
// Half 1 — the type refuses an invalid GTIN through serde
// ---------------------------------------------------------------------------

#[test]
fn deserializing_a_bad_check_digit_fails() {
    let err = serde_json::from_str::<Gtin>(&format!("\"{BAD_CHECK_DIGIT}\""))
        .expect_err("a bad GS1 check digit must not deserialize into a Gtin");
    assert!(
        err.to_string().to_lowercase().contains("check digit"),
        "the rejection should say why; got: {err}"
    );
}

#[test]
fn deserializing_the_valid_form_of_the_same_number_succeeds() {
    // Proves the fixture above fails for its check digit and not because the
    // string is malformed in some other way.
    let gtin = serde_json::from_str::<Gtin>(&format!("\"{GOOD}\""))
        .expect("the corrected check digit must deserialize");
    assert_eq!(gtin.as_str(), GOOD);
}

#[test]
fn an_untyped_payload_reports_no_gtin() {
    // `Other` carries an arbitrary object and identifies no trade item, so it
    // must not answer with one — even when the object happens to hold a `gtin`
    // key. Asserted rather than left implied, because a future `Other` that
    // guessed here would report an unvalidated string as a GTIN.
    let data = ProductGroupData::Other {
        product_group: "hypothetical".to_owned(),
        data: serde_json::json!({ "productGroup": "hypothetical", "gtin": BAD_CHECK_DIGIT }),
    };
    assert_eq!(data.gtin(), None);
}

// ---------------------------------------------------------------------------
// Half 2 — every payload that declares a GTIN uses that type
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is crates/dpp-tests.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/dpp-tests sits two levels below the workspace root")
        .to_path_buf()
}

fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rs_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn every_declared_gtin_field_is_the_validating_type() {
    let root = workspace_root().join("crates/dpp-domain/src/product_group/data");
    let mut files = Vec::new();
    rs_files(&root, &mut files);
    assert!(
        !files.is_empty(),
        "found no payload sources under {}",
        root.display()
    );

    let mut declarations = 0usize;
    let mut wrong: Vec<String> = Vec::new();

    for path in &files {
        let Ok(src) = fs::read_to_string(path) else {
            continue;
        };
        for line in src.lines() {
            let line = line.trim();
            // A struct field declaration, not a doc comment or a match arm.
            let Some(rest) = line.strip_prefix("pub gtin:") else {
                continue;
            };
            declarations += 1;
            let declared = rest.trim().trim_end_matches(',').trim();
            if declared != "Gtin" {
                let name = path.strip_prefix(workspace_root()).unwrap_or(path);
                wrong.push(format!("{}: pub gtin: {declared}", name.display()));
            }
        }
    }

    assert!(
        wrong.is_empty(),
        "\nA payload declares its GTIN as something other than `Gtin`, which is \
         the only type that validates the GS1 check digit:\n\n{}\n\n\
         `String` compiles, satisfies `ProductGroupPayload::gtin` via `as_str()`, \
         and passes the schema's shape pattern — so nothing else would fail.\n",
        wrong.join("\n")
    );

    // A count, so deleting every declaration cannot pass this test by vacuum.
    // Eleven of the twelve typed payloads carry a GTIN; unsold goods is a
    // disclosure over a financial year and identifies no trade item.
    assert_eq!(
        declarations, 11,
        "expected 11 payloads to declare a GTIN, found {declarations} — if a \
         product group was added or removed, update this count deliberately"
    );
}
