//! Tests for the unsold-goods disclosure lints.

use alloc::vec::Vec;

use super::lints::{DisclosureLineInput, UnsoldGoodsLintInput, lint_unsold_goods};

/// A line that breaks nothing: CN heading outside Annex II, split totalling 100.
fn clean_line() -> DisclosureLineInput<'static> {
    DisclosureLineInput {
        cn_category: "6203",
        reason_point: 'f',
        units: 1_200,
        weight_kg: 430,
        preparing_for_reuse_pct: 20,
        recycling_pct: 50,
        other_recovery_pct: 20,
        disposal_pct: 5,
        unknown_pct: 5,
    }
}

fn input<'a>(lines: &'a [DisclosureLineInput<'a>]) -> UnsoldGoodsLintInput<'a> {
    UnsoldGoodsLintInput {
        lines,
        consolidated_undertaking_count: None,
        measures_taken_len: 60,
        measures_planned_len: 60,
    }
}

fn codes(findings: &[super::super::LintFinding]) -> Vec<&str> {
    findings.iter().map(|f| f.code).collect()
}

#[test]
fn a_clean_disclosure_produces_no_findings() {
    let lines = [clean_line()];
    assert!(lint_unsold_goods(&input(&lines)).is_empty());
}

/// Annex I note (i) provides `unknown` for the share that could not be
/// established, so nothing is left over and a split must reach 100.
#[test]
fn a_split_that_misses_100_is_flagged() {
    let mut line = clean_line();
    line.disposal_pct = 1;
    let lines = [line];
    let findings = lint_unsold_goods(&input(&lines));
    assert!(codes(&findings).contains(&"unsold_goods.treatment_split_does_not_total_100"));
}

/// Art. 3: four digits for Annex II products. Chapter 85 holds several, so a
/// chapter-level line there hides which heading applied.
#[test]
fn a_chapter_containing_annex_ii_headings_is_flagged() {
    let mut line = clean_line();
    line.cn_category = "85";
    let lines = [line];
    let findings = lint_unsold_goods(&input(&lines));
    assert!(codes(&findings).contains(&"unsold_goods.cn_category_needs_four_digits"));
}

/// Chapter 62 contains no Annex II heading, so two digits is the depth Art. 3
/// asks for and must not be flagged.
#[test]
fn a_chapter_with_no_annex_ii_heading_is_accepted_at_two_digits() {
    let mut line = clean_line();
    line.cn_category = "62";
    let lines = [line];
    assert!(lint_unsold_goods(&input(&lines)).is_empty());
}

#[test]
fn a_malformed_cn_category_is_flagged_separately() {
    let mut line = clean_line();
    line.cn_category = "620342";
    let lines = [line];
    let findings = lint_unsold_goods(&input(&lines));
    assert!(codes(&findings).contains(&"unsold_goods.cn_category_malformed"));
}

/// Point (h) applies "only where none of the circumstances referred to in points
/// (a) to (g) are applicable".
#[test]
fn point_h_beside_a_stronger_reason_for_the_same_category_is_flagged() {
    let mut donation = clean_line();
    donation.reason_point = 'h';
    let lines = [clean_line(), donation];
    let findings = lint_unsold_goods(&input(&lines));
    assert!(codes(&findings).contains(&"unsold_goods.donation_reason_alongside_stronger_reason"));
}

/// Different categories are independent — (h) for one and (f) for another is
/// exactly what the derogation contemplates.
#[test]
fn point_h_for_a_different_category_is_not_flagged() {
    let mut donation = clean_line();
    donation.reason_point = 'h';
    donation.cn_category = "6204";
    let lines = [clean_line(), donation];
    let findings = lint_unsold_goods(&input(&lines));
    assert!(!codes(&findings).contains(&"unsold_goods.donation_reason_alongside_stronger_reason"));
}

#[test]
fn a_consolidated_disclosure_listing_no_undertakings_is_flagged() {
    let lines = [clean_line()];
    let mut i = input(&lines);
    i.consolidated_undertaking_count = Some(0);
    let findings = lint_unsold_goods(&i);
    assert!(
        codes(&findings).contains(&"unsold_goods.consolidated_disclosure_lists_no_undertakings")
    );
}

#[test]
fn thin_prevention_measures_are_flagged() {
    let lines = [clean_line()];
    let mut i = input(&lines);
    i.measures_planned_len = 3;
    let findings = lint_unsold_goods(&i);
    assert!(codes(&findings).contains(&"unsold_goods.prevention_measures_not_described"));
}

/// Note (f) lets the count be estimated from an accurate weight, so weight with
/// no units at all is a gap rather than a rounding artefact.
#[test]
fn weight_with_no_units_is_flagged() {
    let mut line = clean_line();
    line.units = 0;
    let lines = [line];
    let findings = lint_unsold_goods(&input(&lines));
    assert!(codes(&findings).contains(&"unsold_goods.weight_without_units"));
}
