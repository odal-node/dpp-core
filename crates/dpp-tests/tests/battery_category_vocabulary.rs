//! Tripwire: the Art. 1(3) battery categories say the same thing in all three
//! places that state them.
//!
//! The closed enumeration is written three times and cannot be written once:
//!
//! 1. `crates/dpp-domain/schemas/battery/v*.json` — the `batteryType` `enum`,
//!    which is what a submitted document is validated against;
//! 2. `dpp_domain::product_group::BatteryType` — the serde tags, which decide
//!    what a stored passport deserialises into;
//! 3. `dpp_rules::batteries::category::BATTERY_TYPES` — the wire strings the
//!    Wasm plugins check against, because `plugins/*` are excluded from the
//!    workspace and reach `dpp-rules`, never `dpp-domain`.
//!
//! 🚨 **Two of the three cannot see each other.** A plugin has no way to
//! reference `BatteryType`, and the schema is JSON. So the copies can disagree
//! silently, and the failure is asymmetric: a category the plugin accepts but
//! the domain refuses fails late, at deserialisation; one the plugin refuses
//! but the schema allows rejects a lawful record at the tier that produces the
//! compliance determination.
//!
//! Art. 1(3)'s second subparagraph gives a tie-break — *"the category to which
//! the strictest requirements apply"* — that only functions over a closed set,
//! so "roughly the same five" is not good enough.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use dpp_domain::product_group::BatteryType;
use dpp_rules::batteries::category::BATTERY_TYPES;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/dpp-tests sits two levels below the workspace root")
        .to_path_buf()
}

/// The `batteryType` enum of the newest battery schema.
fn schema_categories(root: &Path) -> BTreeSet<String> {
    let dir = root.join("crates/dpp-domain/schemas/battery");
    let mut versions: Vec<(Vec<u64>, PathBuf)> = fs::read_dir(&dir)
        .expect("the battery schema directory is readable")
        // Not `flatten()`: losing the newest entry would compare the shared
        // vocabulary against a superseded schema and call that agreement.
        .map(|e| e.expect("reading an entry of the battery schema directory"))
        .filter_map(|e| {
            let p = e.path();
            let core = p
                .file_name()?
                .to_str()?
                .strip_prefix('v')?
                .strip_suffix(".json")?;
            let parts: Option<Vec<u64>> = core.split('.').map(|s| s.parse().ok()).collect();
            Some((parts?, p))
        })
        .collect();
    versions.sort();
    let (_, newest) = versions.last().expect("battery ships some schema");
    let doc: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(newest).expect("readable")).expect("JSON");
    doc["properties"]["batteryType"]["enum"]
        .as_array()
        .expect("batteryType declares an enum")
        .iter()
        .filter_map(|v| v.as_str().map(str::to_owned))
        .collect()
}

#[test]
fn the_schema_the_domain_and_the_rules_agree_on_the_five_categories() {
    let shared: BTreeSet<String> = BATTERY_TYPES.iter().map(|s| (*s).to_owned()).collect();
    assert_eq!(shared.len(), 5, "Art. 1(3) enumerates exactly five");

    let domain: BTreeSet<String> = [
        BatteryType::Portable,
        BatteryType::Industrial,
        BatteryType::Ev,
        BatteryType::Lmt,
        BatteryType::Sli,
    ]
    .iter()
    .map(|t| t.wire_str().to_owned())
    .collect();

    assert_eq!(
        shared, domain,
        "dpp_rules::BATTERY_TYPES and BatteryType::wire_str disagree — a \
         plugin would accept a category the domain refuses, or refuse one it \
         accepts"
    );
    assert_eq!(
        shared,
        schema_categories(&workspace_root()),
        "dpp_rules::BATTERY_TYPES and the battery schema's `enum` disagree — \
         one of them is rejecting a record the other considers lawful"
    );
}

#[test]
fn the_domain_wire_tags_are_what_serde_actually_writes() {
    // `wire_str` is a hand-written `match`, so it can drift from the serde
    // attributes it mirrors. The kebab-case rename and the explicit
    // `starting-lighting-ignition` are the two that would drift first.
    for t in [
        BatteryType::Portable,
        BatteryType::Industrial,
        BatteryType::Ev,
        BatteryType::Lmt,
        BatteryType::Sli,
    ] {
        let serialised = serde_json::to_value(&t).expect("a unit variant serialises");
        assert_eq!(
            serialised.as_str(),
            Some(t.wire_str()),
            "{t:?}: wire_str and serde disagree"
        );
    }
}

#[test]
fn sli_is_an_accepted_alias_on_read_but_not_a_category() {
    // `passport_content` tolerates `"sli"` when answering questions about a
    // category. That tolerance is on the read path only — it must not become a
    // value a passport may carry, or two spellings of one category would both
    // be storable and the tie-break in Art. 1(3) would have two inputs.
    assert!(
        !BATTERY_TYPES.contains(&"sli"),
        "`sli` is an alias, not an Art. 1(3) category"
    );
    assert_eq!(BatteryType::Sli.wire_str(), "starting-lighting-ignition");
}
