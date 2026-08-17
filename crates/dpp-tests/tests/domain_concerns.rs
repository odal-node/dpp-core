//! Drift tripwire: the concern inventory in `docs/architecture/ARCHITECTURE.md`
//! must exactly match the public modules declared in `dpp-domain`'s `lib.rs`.
//!
//! `dpp-domain` is the largest crate in the workspace — roughly three and a half
//! times the next one — and is the hub every other crate depends on. That makes
//! it the crate most able to absorb a new capability without anyone noticing,
//! and "it has room" is exactly how a hub becomes a bag.
//!
//! The rule this enforces is that **growing a top-level concern is a deliberate
//! act**: adding one means editing `lib.rs` *and* the inventory, which is the
//! point at which somebody asks whether it belongs. It fails in both directions,
//! so a module removed without updating the docs is caught too.
//!
//! Modelled on `ports_inventory.rs`, which does the same job for `ports/`. That
//! one caught a module being filed as a port when it was not, during this
//! audit — the pattern earns its keep.
//!
//! # Why an inventory rather than a count
//!
//! The architecture record this mirrors described `access` as the crate's
//! "sixth top-level concern". There were already six others alongside it:
//! `compliance` predates that record entirely and was simply not listed in the
//! survey it drew on. Nothing failed, because a prose count has nothing checking
//! it — which is the same defect in miniature that this file exists to prevent,
//! and the reason the doc names the concerns instead of counting them.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

fn manifest_relative(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

/// Public module names declared in `dpp-domain/src/lib.rs` (`pub mod <name>;`).
///
/// `#[cfg(test)] mod test_support` is private and never matches, which is
/// correct: it is not a concern the crate offers anyone.
fn declared_concerns() -> BTreeSet<String> {
    let path = manifest_relative("../dpp-domain/src/lib.rs");
    let src = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    src.lines()
        .map(str::trim)
        .filter_map(|l| l.strip_prefix("pub mod "))
        .filter_map(|l| l.strip_suffix(';'))
        .map(|s| s.trim().to_owned())
        .collect()
}

/// Module names listed in the ARCHITECTURE.md machine block.
fn inventory_concerns() -> BTreeSet<String> {
    let path = manifest_relative("../../docs/architecture/ARCHITECTURE.md");
    let src = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let begin = src
        .find("DOMAIN-CONCERNS:BEGIN")
        .expect("ARCHITECTURE.md missing BEGIN marker");
    let end = src
        .find("DOMAIN-CONCERNS:END")
        .expect("ARCHITECTURE.md missing END marker");
    src[begin..end]
        .lines()
        .skip(1) // the marker line itself
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("```") && !l.starts_with("<!--"))
        .map(str::to_owned)
        .collect()
}

#[test]
fn domain_concerns_match_the_canonical_inventory() {
    let declared = declared_concerns();
    let inventory = inventory_concerns();

    assert!(
        !declared.is_empty(),
        "parsed zero concerns from dpp-domain/src/lib.rs — parser or path is wrong"
    );
    assert_eq!(
        declared,
        inventory,
        "\ndomain concern drift — update docs/architecture/ARCHITECTURE.md to match \
         dpp-domain::lib.\n\
         declared in lib.rs:        {declared:?}\n\
         listed in ARCHITECTURE.md: {inventory:?}\n\
         missing from the doc:      {:?}\n\
         extra in the doc:          {:?}\n\
         \n\
         A new top-level concern in the hub crate is a decision, not a detail. If \
         the module belongs, add it to the inventory in the same commit.\n",
        declared.difference(&inventory).collect::<Vec<_>>(),
        inventory.difference(&declared).collect::<Vec<_>>(),
    );
}
