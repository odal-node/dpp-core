//! `CurrencyState` / `CurrencyCheck` wire shape and predicates, and the
//! currency the shipped instrument manifests actually record.
//!
//! Both live here rather than split across tiers: someone changing the type
//! needs to see every test the change can break, and the manifests are
//! authored against exactly this wire shape.

use super::*;
use crate::instrument::{Instrument, InstrumentCatalog, InstrumentStatus};

/// The flattened wire shape is the contract the manifests are authored against,
/// so it is pinned as a literal rather than round-tripped. A round-trip agrees
/// with itself whatever the shape turns out to be.
#[test]
fn a_check_serialises_flat_with_its_state_tag() {
    let check = CurrencyCheck {
        state: CurrencyState::Consolidated {
            as_of: "02023R1542-20260813".to_owned(),
        },
        checked_on: "2026-09-11".to_owned(),
    };

    assert_eq!(
        serde_json::to_value(&check).expect("serialise"),
        serde_json::json!({
            "state": "consolidated",
            "asOf": "02023R1542-20260813",
            "checkedOn": "2026-09-11",
        })
    );
}

#[test]
fn every_state_round_trips_through_its_wire_form() {
    for state in [
        CurrencyState::InForce,
        CurrencyState::Consolidated {
            as_of: "02024R1781-20240628".to_owned(),
        },
        CurrencyState::Repealed {
            by: "32026R0248".to_owned(),
            on: "2026-02-22".to_owned(),
        },
    ] {
        let check = CurrencyCheck {
            state,
            checked_on: "2026-09-11".to_owned(),
        };
        let json = serde_json::to_string(&check).expect("serialise");
        let back: CurrencyCheck = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(back, check, "round-trip changed {json}");
    }
}

/// A consolidated act is no less binding than an unchanged one — it is quoted
/// from a different text. Folding "changed" into "not law" would drop batteries
/// and ESPR out of every law-shaped question.
#[test]
fn consolidated_is_still_law() {
    assert!(CurrencyState::InForce.is_law());
    assert!(
        CurrencyState::Consolidated {
            as_of: "02023R1542-20260813".to_owned(),
        }
        .is_law()
    );
    assert!(
        !CurrencyState::Repealed {
            by: "32026R0248".to_owned(),
            on: "2026-02-22".to_owned(),
        }
        .is_law()
    );
}

/// A repealed act has no single text to redirect to — which text applied is a
/// question about a date, and this type does not carry one. Answering `Some`
/// here would send a reader to a version that may postdate the conduct they are
/// asking about.
#[test]
fn only_a_consolidation_redirects_the_citation() {
    assert_eq!(CurrencyState::InForce.cite_instead(), None);
    assert_eq!(
        CurrencyState::Consolidated {
            as_of: "02023R1670-20250620".to_owned(),
        }
        .cite_instead(),
        Some("02023R1670-20250620")
    );
    assert_eq!(
        CurrencyState::Repealed {
            by: "32026R0248".to_owned(),
            on: "2026-02-22".to_owned(),
        }
        .cite_instead(),
        None
    );
}

#[test]
fn staleness_compares_iso_dates_lexicographically() {
    let check = CurrencyCheck {
        state: CurrencyState::InForce,
        checked_on: "2026-09-11".to_owned(),
    };

    assert!(check.is_older_than("2026-09-12"));
    assert!(check.is_older_than("2027-01-01"));
    // The boundary is exclusive: a check made on the cutoff is not older than it.
    assert!(!check.is_older_than("2026-09-11"));
    assert!(!check.is_older_than("2026-08-31"));
}

// ── Recorded against the shipped manifests ─────────────────────────

/// The structural half of the currency contract, and the only half that is
/// gated: an adopted act carries a dated check, and nothing else does.
///
/// A proposal is not law and has no in-force status to check; an anticipated act
/// has no text to check at all. Recording a currency against either would be a
/// claim with nothing behind it — the same failure
/// `a_claim_to_have_a_text_is_backed_by_a_celex` guards one axis over.
///
/// This fails when someone adds a manifest, which is a change with an author
/// attached. Staleness is deliberately **not** gated here — see
/// `InstrumentCatalog::currency_checked_before` for why a calendar-triggered
/// failure gets suppressed rather than fixed.
#[test]
fn an_adopted_act_carries_a_dated_currency_check_and_nothing_else_does() {
    for instrument in InstrumentCatalog::new().all() {
        let adopted = instrument.status == InstrumentStatus::Adopted;
        assert_eq!(
            instrument.currency.is_some(),
            adopted,
            "instrument '{}' is {:?} and its currency is {:?}",
            instrument.id,
            instrument.status,
            instrument.currency
        );

        let Some(check) = &instrument.currency else {
            continue;
        };
        let date = &check.checked_on;
        assert!(
            date.len() == 10
                && date.as_bytes()[4] == b'-'
                && date.as_bytes()[7] == b'-'
                && date.chars().filter(char::is_ascii_digit).count() == 8,
            "instrument '{}' has checkedOn '{date}', which is not YYYY-MM-DD — \
             the field is compared as a string and a stray format sorts wrongly \
             rather than failing",
            instrument.id
        );
    }
}

/// A consolidated citation must be a consolidation **of this act**.
///
/// EUR-Lex builds a consolidated CELEX by swapping the sector digit for `0` and
/// appending the version date: `32023R1542` becomes `02023R1542-20260813`. So
/// the relation is checkable, and worth checking — a transposed digit names a
/// real consolidation of a *different* regulation, which resolves, opens, and
/// reads authoritatively. Nothing downstream could catch that.
#[test]
fn a_consolidated_citation_names_a_consolidation_of_its_own_act() {
    for instrument in InstrumentCatalog::new().all() {
        let Some(as_of) = instrument
            .currency
            .as_ref()
            .and_then(|c| c.state.cite_instead())
        else {
            continue;
        };
        let celex = instrument
            .celex
            .as_ref()
            .expect("an adopted act carries a celex");

        let (base, version) = as_of
            .split_once('-')
            .unwrap_or_else(|| panic!("'{}' asOf '{as_of}' has no version date", instrument.id));
        assert_eq!(
            base,
            format!("0{}", &celex[1..]),
            "instrument '{}' cites consolidation '{as_of}', which is not a \
             consolidation of its own act '{celex}'",
            instrument.id
        );
        assert!(
            version.len() == 8 && version.chars().all(|c| c.is_ascii_digit()),
            "instrument '{}' asOf '{as_of}' does not end in a YYYYMMDD version",
            instrument.id
        );
    }
}

/// The two questions the issue behind this field asked to keep apart, shown
/// diverging on one record rather than argued about in prose.
///
/// A repealed act still has a citable text — that is what makes it quotable for
/// history — so `has_citable_text` must stay `true` while `is_current_law` goes
/// `false`. Parsed from JSON rather than built as a literal, which also pins
/// that the wire shape the manifests are authored in actually deserialises.
#[test]
fn a_repealed_act_keeps_its_citable_text_and_loses_its_currency() {
    // CID (EU) 2015/1506, repealed by CIR (EU) 2026/248 — the act this project
    // cited as current law on the day it turned out to have been repealed.
    let repealed: Instrument = serde_json::from_str(
        r#"{
          "id": "ades-formats-2015-1506",
          "title": "Commission Implementing Decision (EU) 2015/1506",
          "celex": "32015D1506",
          "kind": "implementing",
          "status": "adopted",
          "currency": {
            "state": "repealed",
            "by": "32026R0248",
            "on": "2026-02-22",
            "checkedOn": "2026-09-11"
          },
          "parent": "espr",
          "passport": { "obligation": "notRequired" }
        }"#,
    )
    .expect("the manifest wire shape deserialises");

    assert!(
        repealed.status.has_citable_text(),
        "a repealed act still has a text to quote"
    );
    assert!(
        !repealed.is_current_law(),
        "a repealed act may not carry an obligation"
    );

    // And the catalog's own acts go the other way on both.
    for instrument in InstrumentCatalog::new().all() {
        if instrument.status != InstrumentStatus::Adopted {
            continue;
        }
        assert!(
            instrument.is_current_law(),
            "instrument '{}' is adopted but not current law",
            instrument.id
        );
    }
}

/// Unchecked is not current. An adopted act whose currency was never recorded
/// answers `false`, for the same reason an unmarked retention figure is
/// `Assumed`: the safe default for a claim about someone else's legal
/// obligation is the one that refuses to assert it.
#[test]
fn an_adopted_act_with_no_recorded_check_is_not_current_law() {
    let mut unchecked = InstrumentCatalog::new()
        .get("battery-reg-2023-1542")
        .expect("battery is embedded")
        .clone();
    assert!(unchecked.is_current_law());

    unchecked.currency = None;
    assert!(
        !unchecked.is_current_law(),
        "an unrecorded check must not read as a current one"
    );
}

/// The staleness query reports both halves a report needs: checks older than
/// the cutoff, and adopted acts never checked at all.
#[test]
fn the_staleness_query_reports_stale_and_never_checked_alike() {
    let catalog = InstrumentCatalog::new();

    // Every embedded check was made on 2026-09-11.
    assert!(
        catalog.currency_checked_before("2026-09-11").is_empty(),
        "the cutoff is exclusive, so a check made on it is not yet stale"
    );

    let stale = catalog.currency_checked_before("2026-09-12");
    let adopted = catalog
        .all()
        .iter()
        .filter(|i| i.status == InstrumentStatus::Adopted)
        .count();
    assert_eq!(
        stale.len(),
        adopted,
        "every adopted act was checked before the cutoff"
    );

    // An act with no check at all is reported however early the cutoff.
    let mut catalog = InstrumentCatalog::new();
    let mut never = catalog.get("espr").expect("espr is embedded").clone();
    never.id = "never-checked".to_owned();
    never.currency = None;
    catalog.register(never).expect("a fresh id");
    assert!(
        catalog
            .currency_checked_before("1970-01-01")
            .iter()
            .any(|i| i.id == "never-checked"),
        "an adopted act that was never checked must appear in every report"
    );
}
