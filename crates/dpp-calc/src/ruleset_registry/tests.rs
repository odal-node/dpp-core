//! Tests for ruleset resolution and the calculator status map.

use super::*;
use crate::assessability::Assessability;
use crate::ruleset::Effectivity;

/// The wall-clock date, used only where the question genuinely is "what can be
/// computed today" — a readiness view, never a compliance determination.
fn today() -> chrono::NaiveDate {
    Utc::now().date_naive()
}
use chrono::{NaiveDate, Utc};

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

#[test]
fn smartphone_heuristic_available_after_june_2025() {
    assert!(
        resolve_repairability("smartphone-tablet", date(2025, 6, 20)).is_assessed(),
        "heuristic should resolve on/after its availability date (20 Jun 2025)"
    );
    assert!(
        resolve_repairability("smartphone-tablet", date(2026, 1, 1)).is_assessed(),
        "heuristic should resolve the following year"
    );
}

#[test]
fn smartphone_heuristic_unavailable_before_june_2025() {
    // Not merely absent — the caller is told the date it starts.
    let got = resolve_repairability("smartphone-tablet", date(2025, 5, 31));
    let Assessability::NotYetInForce {
        ruleset_id,
        applies_from,
    } = got
    else {
        panic!("expected NotYetInForce");
    };
    assert_eq!(ruleset_id, "repairability-heuristic-v1");
    assert_eq!(applies_from, date(2025, 6, 20));
}

#[test]
fn laptop_is_undetermined_not_merely_absent() {
    // LaptopRuleset is Pending, so the answer names the instrument being waited
    // on rather than returning a bare "nothing found".
    let got = resolve_repairability("laptop", date(2026, 6, 9));
    let Assessability::Undetermined {
        ruleset_id,
        empowerment,
    } = got
    else {
        panic!("expected Undetermined");
    };
    assert_eq!(ruleset_id, "laptop-repairability");
    assert!(
        empowerment.contains("2024/1781"),
        "empowerment should cite the instrument: {empowerment}"
    );
}

#[test]
fn unknown_category_is_out_of_scope() {
    // A category with no row at all. An earlier version of this test passed
    // "washing-machine" — a *known* category with a pending ruleset — so it
    // asserted the right outcome for the wrong reason, and `Option` could not
    // tell the two apart.
    assert!(matches!(
        resolve_repairability("kettle", date(2026, 1, 1)),
        Assessability::OutOfScope
    ));
}

/// CI invariant: any ruleset whose effective period has already ended must
/// declare a successor via `regulatory_basis().superseded_by`.
///
/// This prevents a future engineer from expiring a ruleset and forgetting
/// to point auditors at the replacement — which would leave receipts from
/// that period referencing a dead-end in the regulatory chain.
#[test]
fn expired_rulesets_have_superseded_by() {
    let today = Utc::now().date_naive();
    for r in all_rulesets() {
        if let Effectivity::InForce {
            until: Some(until), ..
        } = r.effectivity()
            && *until < today
        {
            assert!(
                r.regulatory_basis().superseded_by.is_some(),
                "ruleset '{}' expired on {until} but regulatory_basis.superseded_by is None",
                r.id().0,
            );
        }
    }
}

#[test]
fn resolved_ruleset_carries_non_regulatory_basis() {
    // The repairability heuristic must NOT claim EU 2023/1669 conformance.
    let r = resolve_repairability("smartphone-tablet", date(2026, 1, 1))
        .assessed()
        .expect("should resolve");
    let basis = r.regulatory_basis();
    assert!(
        basis.regulation.contains("Non-regulatory")
            && basis.regulation.contains("NOT EU 2023/1669"),
        "heuristic basis must disclaim regulatory conformance, got {:?}",
        basis.regulation
    );
    assert!(
        basis.standard.is_none(),
        "heuristic must not claim a harmonised standard"
    );
}

#[test]
fn displays_not_active_today() {
    assert!(
        matches!(
            resolve_repairability("displays", date(2026, 6, 16)),
            Assessability::Undetermined { .. }
        ),
        "displays ruleset awaits an unadopted act"
    );
}

#[test]
fn washing_machine_not_active_today() {
    assert!(
        matches!(
            resolve_repairability("washing-machine", date(2026, 6, 16)),
            Assessability::Undetermined { .. }
        ),
        "washing-machine ruleset awaits an unadopted act"
    );
}

// ── sector_calculator_map ─────────────────────────────────────────────

#[test]
fn sector_calculator_map_has_at_least_one_active_entry() {
    let map = sector_calculator_map();
    assert!(
        map.iter()
            .any(|e| matches!(e.status_on(today()), CalculatorStatus::Active { .. })),
        "expected at least one Active entry in the calculator map"
    );
}

#[test]
fn smartphone_tablet_is_active_in_map() {
    let map = sector_calculator_map();
    let entry = map
        .iter()
        .find(|e| e.product_category == "smartphone-tablet")
        .expect("smartphone-tablet must appear in the map");
    assert!(
        matches!(
            entry.status_on(today()),
            CalculatorStatus::Active {
                ruleset_id: "repairability-heuristic-v1"
            }
        ),
        "expected Active, got {:?}",
        entry.status_on(today())
    );
}

#[test]
fn battery_pef_is_active_in_map() {
    let map = sector_calculator_map();
    let entry = map
        .iter()
        .find(|e| e.sector_key == "battery" && e.methodology == "co2e-pef")
        .expect("battery co2e-pef must appear in the map");
    assert!(matches!(
        entry.status_on(today()),
        CalculatorStatus::Active { .. }
    ));
}

#[test]
fn all_sector_calculator_entries_have_non_empty_strings() {
    for entry in sector_calculator_map() {
        assert!(!entry.sector_key.is_empty(), "sector_key is empty");
        assert!(
            !entry.product_category.is_empty(),
            "product_category is empty"
        );
        assert!(!entry.methodology.is_empty(), "methodology is empty");
    }
}

/// Drift guard: `sector_calculator_map()` and `resolve_repairability`'s
/// internal category table are hand-maintained independently, with nothing
/// else tying them together — a category added to one and forgotten in the
/// other fails silently (the status view is simply wrong, not a crash). For
/// every repairability-heuristic entry, its declared status must agree with
/// what `resolve_repairability` actually resolves today.
#[test]
fn status_map_agrees_with_resolve_repairability_today() {
    let today = Utc::now().date_naive();
    for entry in sector_calculator_map() {
        if entry.methodology != "repairability-heuristic" {
            continue;
        }
        let resolves_today = resolve_repairability(entry.product_category, today).is_assessed();
        match entry.status_on(today) {
            CalculatorStatus::Active { .. } => assert!(
                resolves_today,
                "sector_calculator_map derives Active for '{}', but resolve_repairability \
                 finds no ruleset for it today",
                entry.product_category
            ),
            other => assert!(
                !resolves_today,
                "sector_calculator_map derives {other:?} for '{}', but resolve_repairability \
                 already resolves a ruleset for it today",
                entry.product_category
            ),
        }
    }
}

/// The same drift guard for the Art. 8 entries.
///
/// These two tables are hand-maintained independently, and the LMT row is the
/// one most able to go wrong quietly: Art. 8(2) does not reach LMT batteries, so
/// a map entry pointing them at the Phase 1 ruleset would report a status for a
/// rule the resolver will never hand them.
#[test]
fn status_map_agrees_with_resolve_recycled_content_today() {
    let today = Utc::now().date_naive();
    for entry in sector_calculator_map() {
        if entry.methodology != "recycled-content-art8" {
            continue;
        }
        let resolves_today =
            crate::ruleset_registry::resolve_recycled_content(entry.product_category, today)
                .is_assessed();
        match entry.status_on(today) {
            CalculatorStatus::Active { .. } => assert!(
                resolves_today,
                "map derives Active for '{}', but resolve_recycled_content finds nothing",
                entry.product_category
            ),
            other => assert!(
                !resolves_today,
                "map derives {other:?} for '{}', but resolve_recycled_content already resolves one",
                entry.product_category
            ),
        }
    }
}

/// Every `Active` map entry must reference a `ruleset_id` that actually exists
/// in `all_rulesets()`. Guards against the map and the concrete `Ruleset::id()`
/// drifting apart (e.g. map saying `"cradle-to-gate-pef"` while the impl returns
/// `"co2e-cradle-to-gate"`), which would make a lookup-by-id silently fail.
#[test]
fn active_map_entries_reference_real_rulesets() {
    let known: std::collections::HashSet<&str> = all_rulesets().iter().map(|r| r.id().0).collect();
    for entry in sector_calculator_map() {
        if let CalculatorImpl::Ruleset(ruleset_id) = &entry.implementation {
            assert!(
                known.contains(ruleset_id),
                "map entry ({}/{}) references ruleset_id '{ruleset_id}' not present in all_rulesets()",
                entry.sector_key,
                entry.product_category,
            );
        }
    }
}
