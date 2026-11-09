//! Tests for the unsold-goods disclosure lints.

use alloc::vec::Vec;

use super::lints::{DisclosureLineInput, UnsoldGoodsLintInput, lint_unsold_goods};
use super::records::{DiscardRecordLintInput, lint_discard_record};

/// A line that breaks nothing: CN heading outside Annex II, split totalling 100.
fn clean_line() -> DisclosureLineInput<'static> {
    DisclosureLineInput {
        cn_category: "6203",
        reason_point: Some('f'),
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
        financial_year_start: (2028, 1, 1),
        financial_year_end: (2028, 12, 31),
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
    donation.reason_point = Some('h');
    let lines = [clean_line(), donation];
    let findings = lint_unsold_goods(&input(&lines));
    assert!(codes(&findings).contains(&"unsold_goods.donation_reason_alongside_stronger_reason"));
}

/// Different categories are independent — (h) for one and (f) for another is
/// exactly what the derogation contemplates.
#[test]
fn point_h_for_a_different_category_is_not_flagged() {
    let mut donation = clean_line();
    donation.reason_point = Some('h');
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

/// A line with no derogation is a real answer for a product no derogation
/// reaches, so it must not trip the point (h) check that reads reasons.
#[test]
fn a_line_with_no_derogation_is_not_read_as_a_donation_reason() {
    let mut none = clean_line();
    none.reason_point = None;
    let lines = [clean_line(), none];
    assert!(lint_unsold_goods(&input(&lines)).is_empty());
}

/// `FinancialYear` leaves a start after its end unrefused, on the promise that
/// it is reported here.
#[test]
fn a_financial_year_ending_before_it_starts_is_flagged() {
    let lines = [clean_line()];
    let mut i = input(&lines);
    i.financial_year_start = (2028, 12, 31);
    i.financial_year_end = (2028, 1, 1);
    let findings = lint_unsold_goods(&i);
    assert!(codes(&findings).contains(&"unsold_goods.financial_year_ends_before_it_starts"));
}

/// A one-day year is odd but not backwards.
#[test]
fn a_financial_year_that_starts_and_ends_on_one_day_is_not_flagged() {
    let lines = [clean_line()];
    let mut i = input(&lines);
    i.financial_year_start = (2028, 6, 30);
    i.financial_year_end = (2028, 6, 30);
    assert!(lint_unsold_goods(&i).is_empty());
}

/// A destruction of apparel with a derogation, weighed, with a readable code.
fn clean_record() -> DiscardRecordLintInput<'static> {
    DiscardRecordLintInput {
        cn_codes: &["62034200"],
        reason_point: Some('f'),
        prepared_for_reuse: false,
        weight_grams: 12_000,
    }
}

#[test]
fn a_clean_record_produces_no_findings() {
    assert!(lint_discard_record(&clean_record()).is_empty());
}

/// ESPR Art. 25(1) prohibits destroying Annex VII goods and DR 2026/296 Art. 2
/// permits it only under a point.
#[test]
fn an_annex_vii_discard_with_no_derogation_is_flagged() {
    let mut record = clean_record();
    record.reason_point = None;
    let findings = lint_discard_record(&record);
    assert_eq!(
        codes(&findings),
        ["unsold_goods.annex_vii_discard_without_derogation"]
    );
}

/// Soap is not in Annex VII; no prohibition, so no derogation is owed.
#[test]
fn a_discard_outside_annex_vii_needs_no_derogation() {
    let mut record = clean_record();
    record.cn_codes = &["34011100"];
    record.reason_point = None;
    assert!(lint_discard_record(&record).is_empty());
}

/// ESPR Art. 2(34): discarding for the sole purpose of preparing for reuse is
/// not destruction, so the prohibition does not reach it.
#[test]
fn an_annex_vii_discard_for_reuse_needs_no_derogation() {
    let mut record = clean_record();
    record.reason_point = None;
    record.prepared_for_reuse = true;
    assert!(lint_discard_record(&record).is_empty());
}

#[test]
fn a_derogation_on_a_discard_for_reuse_is_noticed() {
    let mut record = clean_record();
    record.prepared_for_reuse = true;
    let findings = lint_discard_record(&record);
    assert_eq!(
        codes(&findings),
        ["unsold_goods.derogation_without_annex_vii_destruction"]
    );
}

#[test]
fn a_derogation_on_a_good_outside_annex_vii_is_noticed() {
    let mut record = clean_record();
    record.cn_codes = &["34011100"];
    let findings = lint_discard_record(&record);
    assert_eq!(
        codes(&findings),
        ["unsold_goods.derogation_without_annex_vii_destruction"]
    );
}

/// A kit may carry several CN codes (note (f)); one in Annex VII is enough.
#[test]
fn one_annex_vii_code_among_several_puts_a_kit_in_scope() {
    let mut record = clean_record();
    record.cn_codes = &["34011100", "62034200"];
    record.reason_point = None;
    let findings = lint_discard_record(&record);
    assert_eq!(
        codes(&findings),
        ["unsold_goods.annex_vii_discard_without_derogation"]
    );
}

#[test]
fn a_record_with_no_cn_code_is_flagged() {
    let mut record = clean_record();
    record.cn_codes = &[];
    let findings = lint_discard_record(&record);
    assert_eq!(codes(&findings), ["unsold_goods.record_cn_code_unreadable"]);
}

#[test]
fn a_record_with_a_cn_code_of_the_wrong_length_is_flagged() {
    let mut record = clean_record();
    record.cn_codes = &["6203"];
    let findings = lint_discard_record(&record);
    assert_eq!(codes(&findings), ["unsold_goods.record_cn_code_unreadable"]);
}

/// Note (g) lets the weight be estimated from the count, so zero means the
/// figure was left out, not that nothing weighed anything.
#[test]
fn a_record_without_weight_is_flagged() {
    let mut record = clean_record();
    record.weight_grams = 0;
    let findings = lint_discard_record(&record);
    assert_eq!(codes(&findings), ["unsold_goods.record_without_weight"]);
}
