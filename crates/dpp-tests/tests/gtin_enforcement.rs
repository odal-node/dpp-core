//! Tripwire: a payload's product identifier keeps the type that validates it.
//!
//! GTIN validity across every product group rests on one declaration per
//! payload — `product_identifier: ProductIdentifier`, whose scheme 1 arm holds a
//! `Gtin` — plus `Gtin`'s hand-written `Deserialize`, which calls `Gtin::parse`.
//! Together those refuse a bad GS1 check digit while a document is being
//! deserialised, for every product group at once, before any caller sees the
//! value.
//!
//! Nothing asserted that the arrangement holds, and it fails silently. Change
//! one payload's field to a bare `String` and validation for that product group
//! disappears without anything going red: the crate compiles, the schema's
//! digit pattern still passes because it checks shape rather than the check
//! digit, and the payload still answers with a string. Every fixture in this
//! workspace builds its identifier through `Gtin::parse`, so all of them are
//! valid by construction and none of them could catch it.
//!
//! The invariant has two halves and this file pins both:
//!
//! 1. **The type refuses an invalid GTIN through serde** — behavioural, tested
//!    directly on `Gtin`, so it needs no payload fixture.
//! 2. **Every payload declares its identifier as `ProductIdentifier`** —
//!    structural, read from the source, so a new product group cannot be added
//!    without one.
//!
//! Neither half is sufficient alone: the first says the lock works, the second
//! says every door has it fitted.
//!
//! 🚨 The second half now guards a *reachability* claim as well as a type. The
//! check digit is validated inside `ProductIdentifier`'s scheme 1 arm, so a
//! payload that declared some other identifier type would lose the validation
//! without losing a field — which is exactly the silent failure above, one
//! level further in.

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
    // `Other` answers only with a `productIdentifier` that parses as an
    // EN 18219 clause 5 identifier, so a bare `gtin` key — here with a bad
    // check digit — must not make it answer. Asserted rather than left implied,
    // because an `Other` that guessed here would report an unvalidated string
    // as a GTIN.
    let data = ProductGroupData::other(
        serde_json::json!({ "productGroup": "hypothetical", "gtin": BAD_CHECK_DIGIT }),
    )
    .expect("an untyped product group");
    assert_eq!(data.gtin(), None);
    // 🚨 And the question that actually distinguishes them. `gtin()` answers
    // `None` for a scheme 2 or 3 payload too, so it can no longer tell "no
    // identifier" from "an identifier with no GTIN" — only this can, which is
    // why `Other` must answer `None` here and not merely there.
    assert_eq!(data.product_identifier(), None);
}

// ---------------------------------------------------------------------------
// Half 2 — every payload declares its identifier as the validating type
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
fn every_payload_identifier_field_is_the_validating_type() {
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
            let Some(rest) = line.strip_prefix("pub product_identifier:") else {
                continue;
            };
            declarations += 1;
            let declared = rest.trim().trim_end_matches(',').trim();
            if declared != "ProductIdentifier" {
                let name = path.strip_prefix(workspace_root()).unwrap_or(path);
                wrong.push(format!(
                    "{}: pub product_identifier: {declared}",
                    name.display()
                ));
            }
        }
    }

    assert!(
        wrong.is_empty(),
        "\nA payload declares its identifier as something other than \
         `ProductIdentifier`, which is the only type that validates it — the \
         GS1 check digit inside its scheme 1 arm, and the URL and DID syntax \
         in the other two:\n\n{}\n\n\
         `String` compiles, satisfies `ProductGroupPayload::product_identifier` \
         callers that only read it back out, and passes the schema's shape \
         pattern — so nothing else would fail.\n",
        wrong.join("\n")
    );

    // A count, so deleting every declaration cannot pass this test by vacuum.
    // Eleven of the twelve typed payloads carry a product identifier; unsold
    // goods is a disclosure over a financial year and identifies no trade item.
    //
    // 🚨 "Carries a product identifier" is not "carries a GTIN". Only scheme 1
    // has one; schemes 2 and 3 are self-issuing and have none, so a count of
    // GTINs would be a count of nothing in particular.
    assert_eq!(
        declarations, 11,
        "expected 11 payloads to declare a product identifier, found \
         {declarations} — if a product group was added or removed, update this \
         count deliberately"
    );
}
