//! Tests for ruleset resolution and the calculator status map.

use super::*;
use chrono::{NaiveDate, Utc};

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

#[test]
fn smartphone_heuristic_available_after_june_2025() {
    assert!(
        resolve_repairability("smartphone-tablet", date(2025, 6, 20)).is_some(),
        "heuristic should resolve on/after its availability date (20 Jun 2025)"
    );
    assert!(
        resolve_repairability("smartphone-tablet", date(2026, 1, 1)).is_some(),
        "heuristic should resolve the following year"
    );
}

#[test]
fn smartphone_heuristic_unavailable_before_june_2025() {
    assert!(
        resolve_repairability("smartphone-tablet", date(2025, 5, 31)).is_none(),
        "heuristic must not resolve before its availability date"
    );
}

#[test]
fn laptop_not_active_today() {
    // LaptopRuleset has from=2100 — no date today should resolve it.
    assert!(
        resolve_repairability("laptop", date(2026, 6, 9)).is_none(),
        "laptop ruleset is a 2100 stub and must not resolve before then"
    );
}

#[test]
fn unknown_category_returns_none() {
    assert!(resolve_repairability("washing-machine", date(2026, 1, 1)).is_none());
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
        if let Some(until) = r.effective_dates().until
            && until < today
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
    let r = resolve_repairability("smartphone-tablet", date(2026, 1, 1)).expect("should resolve");
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
        resolve_repairability("displays", date(2026, 6, 16)).is_none(),
        "displays ruleset is a stub and must not resolve before 2100"
    );
}

#[test]
fn washing_machine_not_active_today() {
    assert!(
        resolve_repairability("washing-machine", date(2026, 6, 16)).is_none(),
        "washing-machine ruleset is a stub and must not resolve before 2100"
    );
}

// ── sector_calculator_map ─────────────────────────────────────────────

#[test]
fn sector_calculator_map_has_at_least_one_active_entry() {
    let map = sector_calculator_map();
    assert!(
        map.iter()
            .any(|e| matches!(e.status, CalculatorStatus::Active { .. })),
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
            &entry.status,
            CalculatorStatus::Active {
                ruleset_id: "repairability-heuristic-v1"
            }
        ),
        "expected Active, got {:?}",
        entry.status
    );
}

#[test]
fn battery_pef_is_active_in_map() {
    let map = sector_calculator_map();
    let entry = map
        .iter()
        .find(|e| e.sector_key == "battery" && e.methodology == "co2e-pef")
        .expect("battery co2e-pef must appear in the map");
    assert!(matches!(entry.status, CalculatorStatus::Active { .. }));
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
        let resolves_today = resolve_repairability(entry.product_category, today).is_some();
        match &entry.status {
            CalculatorStatus::Active { .. } => assert!(
                resolves_today,
                "sector_calculator_map marks '{}' Active, but resolve_repairability finds \
                 no ruleset for it today",
                entry.product_category
            ),
            CalculatorStatus::PendingDelegatedAct { .. } => assert!(
                !resolves_today,
                "sector_calculator_map marks '{}' PendingDelegatedAct, but \
                 resolve_repairability already resolves a ruleset for it today",
                entry.product_category
            ),
            CalculatorStatus::ReportingOnly => {}
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
        if let CalculatorStatus::Active { ruleset_id } = &entry.status {
            assert!(
                known.contains(ruleset_id),
                "map entry ({}/{}) references ruleset_id '{ruleset_id}' not present in all_rulesets()",
                entry.sector_key,
                entry.product_category,
            );
        }
    }
}
