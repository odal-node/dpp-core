//! Tests for the unsold-goods disclosure model.

use chrono::NaiveDate;
use serde_json::json;

use super::*;

fn line() -> DiscardedProductLine {
    DiscardedProductLine {
        cn_categories: vec![CnCategory::parse("6203").unwrap()],
        description: "Men's suits, ensembles, jackets and trousers".to_owned(),
        units_discarded: DiscardedQuantity::measured(1_200),
        weight_kg: DiscardedQuantity::estimated(430),
        packaging_included: false,
        reason: DiscardReason::DamagedOrContaminated,
        reason_detail: None,
        treatment: WasteTreatmentSplit {
            preparing_for_reuse_pct: 20,
            recycling_pct: 50,
            other_recovery_pct: 20,
            disposal_pct: 5,
            unknown_pct: 5,
        },
    }
}

fn report() -> UnsoldGoodsReport {
    UnsoldGoodsReport {
        entity: DisclosingEntity {
            name: "Example Retail Group SA".to_owned(),
            identifier: LegalEntityIdentifier::Euid {
                value: "LUB123456789".to_owned(),
            },
            scope: DisclosureScope::Standalone,
        },
        financial_year: FinancialYear {
            start: NaiveDate::from_ymd_opt(2027, 4, 1).unwrap(),
            end: NaiveDate::from_ymd_opt(2028, 3, 31).unwrap(),
        },
        lines: vec![line()],
        measures_taken: "Introduced pre-season demand forecasting.".to_owned(),
        measures_planned: "Extending the donation window to twelve weeks.".to_owned(),
    }
}

// ── CN category ─────────────────────────────────────────────────────────────

/// Art. 3 allows exactly two depths: the CN chapter and the CN heading.
#[test]
fn both_depths_article_3_allows_parse() {
    let chapter = CnCategory::parse("62").expect("chapter");
    assert_eq!(chapter.as_str(), "62");
    assert!(!chapter.is_heading());

    let heading = CnCategory::parse("6203").expect("heading");
    assert_eq!(heading.as_str(), "6203");
    assert!(heading.is_heading());
    assert_eq!(heading.chapter(), "62");
}

/// A product's own 6/8/10-digit code is not a disclosure category. Accepting one
/// would file a whole chapter's worth of goods under a single article.
#[test]
fn a_commodity_code_is_not_a_cn_category() {
    for code in ["620342", "62034231", "6203423100"] {
        assert!(
            CnCategory::parse(code).is_err(),
            "{code} must not parse as a CN category"
        );
    }
}

/// Compacting `"62 03"` would turn a mistyped value into a different, valid
/// heading — so separators are refused, never stripped.
#[test]
fn separators_are_refused_not_stripped() {
    for code in ["62 03", "62.03", "62-03", ""] {
        assert!(CnCategory::parse(code).is_err(), "{code} must not parse");
    }
    assert_eq!(CnCategory::parse("  6203 ").unwrap().as_str(), "6203");
}

// ── The reason vocabulary ───────────────────────────────────────────────────

/// Del. Reg. (EU) 2026/296 Art. 2 enumerates points (a) to (j). If the list here
/// is ever a different length, one of them has been dropped or invented.
#[test]
fn the_reason_list_is_article_2_points_a_to_j() {
    assert_eq!(DiscardReason::ALL.len(), 10);
    let points: Vec<char> = DiscardReason::ALL
        .iter()
        .map(|r| r.article_2_point())
        .collect();
    assert_eq!(
        points,
        vec!['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j']
    );
}

/// Point (h) alone begins "only where none of the circumstances referred to in
/// points (a) to (g) are applicable".
#[test]
fn only_point_h_is_subordinate() {
    let subordinate: Vec<char> = DiscardReason::ALL
        .iter()
        .filter(|r| r.is_subordinate())
        .map(|r| r.article_2_point())
        .collect();
    assert_eq!(subordinate, vec!['h']);
}

// ── The treatment split ─────────────────────────────────────────────────────

/// Annex I note (i): "Destruction is the sum of recycling, other recovery and
/// disposal." Preparing for reuse and unknown are outside it — which is not the
/// intuitive reading, and is why this is asserted rather than assumed.
#[test]
fn destruction_is_recycling_plus_other_recovery_plus_disposal() {
    let split = line().treatment;
    assert_eq!(split.total_destruction_pct(), 75);
    assert_eq!(split.total_pct(), 100);
}

/// The shares are `u8`, so three of them can exceed `u8::MAX` in a malformed
/// record. The sum widens rather than wrapping or panicking: an impossible
/// number a caller can see beats a plausible one it cannot.
#[test]
fn a_malformed_split_widens_rather_than_wrapping() {
    let split = WasteTreatmentSplit {
        preparing_for_reuse_pct: 0,
        recycling_pct: 200,
        other_recovery_pct: 200,
        disposal_pct: 200,
        unknown_pct: 0,
    };
    assert_eq!(split.total_destruction_pct(), 600);
}

// ── Quantities ──────────────────────────────────────────────────────────────

/// Annex I notes (f) and (g): an estimate is shown "accompanying the disclosed
/// value with `±`", and Section 2 forbids separators.
#[test]
fn an_estimate_renders_with_the_annex_i_marker() {
    assert_eq!(DiscardedQuantity::estimated(430).to_string(), "±430");
    assert_eq!(DiscardedQuantity::measured(1_200).to_string(), "1200");
}

// ── The report ──────────────────────────────────────────────────────────────

/// Art. 1: disclosure is due "within 12 months after the end of that financial
/// year".
#[test]
fn the_disclosure_deadline_is_twelve_months_after_the_year_end() {
    let due = report().financial_year.disclosure_due_by().unwrap();
    assert_eq!(due, NaiveDate::from_ymd_opt(2029, 3, 31).unwrap());
}

#[test]
fn totals_aggregate_across_lines() {
    let mut r = report();
    r.lines.push(line());
    assert_eq!(r.total_units(), 2_400);
    assert_eq!(r.total_weight_kg(), 860);
    assert!(r.contains_estimates());
}

// ── Wire format ─────────────────────────────────────────────────────────────

#[test]
fn the_report_round_trips() {
    let original = report();
    let json = serde_json::to_string(&original).expect("serialise");
    let back: UnsoldGoodsReport = serde_json::from_str(&json).expect("deserialise");
    assert_eq!(back, original);
}

/// The wire is camelCase and the two tagged enums carry a `type` discriminant,
/// which is what Annex I's checkbox rows become.
#[test]
fn the_wire_keys_are_camel_case_and_tagged() {
    let value = serde_json::to_value(report()).unwrap();
    assert_eq!(value["financialYear"]["start"], json!("2027-04-01"));
    assert_eq!(value["entity"]["identifier"]["type"], json!("euid"));
    assert_eq!(value["entity"]["scope"]["type"], json!("standalone"));
    assert!(value["measuresTaken"].is_string());

    let line = &value["lines"][0];
    assert_eq!(line["cnCategories"], json!(["6203"]));
    assert_eq!(line["packagingIncluded"], json!(false));
    assert_eq!(line["reason"], json!("damagedOrContaminated"));
    assert_eq!(line["weightKg"]["estimated"], json!(true));
    assert_eq!(line["treatment"]["recyclingPct"], json!(50));
}

/// Total destruction is derived from three fields the act defines it in terms
/// of, so it must not appear on the wire as a fourth.
#[test]
fn total_destruction_is_not_a_stored_field() {
    let value = serde_json::to_value(report()).unwrap();
    let treatment = &value["lines"][0]["treatment"];
    assert!(
        treatment.get("totalDestructionPct").is_none(),
        "totalDestructionPct must be derived, not stored: {treatment}"
    );
}

/// A consolidated disclosure has to name its undertakings — note (c) — and the
/// standalone arm has nowhere to put them.
#[test]
fn a_consolidated_scope_carries_its_undertakings() {
    let scope = DisclosureScope::Consolidated {
        undertakings: vec!["Sub One SARL".to_owned(), "Sub Two GmbH".to_owned()],
    };
    let value = serde_json::to_value(&scope).unwrap();
    assert_eq!(value["type"], json!("consolidated"));
    assert_eq!(value["undertakings"][1], json!("Sub Two GmbH"));

    let back: DisclosureScope = serde_json::from_value(value).unwrap();
    assert_eq!(back, scope);
}

/// Where no EUID is available the scheme has to travel with the value, or a
/// reader cannot resolve it.
#[test]
fn a_non_euid_identifier_carries_its_scheme() {
    let id = LegalEntityIdentifier::Other {
        scheme: "SE-Bolagsverket".to_owned(),
        value: "5560000000".to_owned(),
    };
    assert_eq!(id.value(), "5560000000");
    let value = serde_json::to_value(&id).unwrap();
    assert_eq!(value["type"], json!("other"));
    assert_eq!(value["scheme"], json!("SE-Bolagsverket"));
}
