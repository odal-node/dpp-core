//! Drift tripwire: the port inventory in `docs/architecture/PORTS.md`
//! must exactly match the modules declared in `dpp-domain::ports`.
//!
//! This kills the "six port traits" class of doc/code drift mechanically —
//! adding or removing a port without updating the canonical inventory turns CI
//! red in either direction. Docs quote PORTS.md; they never quote a number.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

fn manifest_relative(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

/// Module names declared in `dpp-domain/src/ports/mod.rs` (`pub mod <name>;`).
fn declared_ports() -> BTreeSet<String> {
    let path = manifest_relative("../dpp-domain/src/ports/mod.rs");
    let src = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    src.lines()
        .map(str::trim)
        .filter_map(|l| l.strip_prefix("pub mod "))
        .filter_map(|l| l.strip_suffix(';'))
        .map(|s| s.trim().to_owned())
        .collect()
}

/// Module names listed in the PORTS.md machine block.
fn inventory_ports() -> BTreeSet<String> {
    let path = manifest_relative("../../docs/architecture/PORTS.md");
    let src = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let begin = src
        .find("PORTS-INVENTORY:BEGIN")
        .expect("PORTS.md missing BEGIN marker");
    let end = src
        .find("PORTS-INVENTORY:END")
        .expect("PORTS.md missing END marker");
    src[begin..end]
        .lines()
        .skip(1) // the marker line itself
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("```") && !l.starts_with("<!--"))
        .map(str::to_owned)
        .collect()
}

#[test]
fn ports_module_matches_canonical_inventory() {
    let declared = declared_ports();
    let inventory = inventory_ports();

    assert!(
        !declared.is_empty(),
        "parsed zero ports from ports/mod.rs — parser or path is wrong"
    );
    assert_eq!(
        declared,
        inventory,
        "\nport inventory drift — update docs/architecture/PORTS.md to match dpp-domain::ports.\n\
         declared in ports/mod.rs: {declared:?}\n\
         listed in PORTS.md:       {inventory:?}\n\
         missing from PORTS.md:    {:?}\n\
         extra in PORTS.md:        {:?}\n",
        declared.difference(&inventory).collect::<Vec<_>>(),
        inventory.difference(&declared).collect::<Vec<_>>(),
    );
}
