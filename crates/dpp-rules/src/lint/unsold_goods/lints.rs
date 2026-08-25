//! Unsold-goods disclosure lints — consistency checks that Commission
//! Implementing Regulation (EU) 2026/2 implies but a JSON Schema cannot state.
//!
//! Advisory only. A lint never blocks a disclosure; it says something looks
//! wrong to a reader who knows the act.
//!
//! # Replaced wholesale
//!
//! The previous pack linted a shape that predated the act: a `"YYYY-QN"`
//! reporting period, a single `volume_kg`, one destination word, and a free-text
//! destruction justification. None of those fields exists any more, and three of
//! its five findings were about a quarter — a period this disclosure is never
//! made for, since Art. 1 fixes it to the undertaking's **financial year**.

use alloc::{format, vec::Vec};

use super::super::{LintFinding, LintSeverity};
use crate::unsold_goods::disclosure;

/// Borrowing view over one disclosure line.
#[derive(Debug, Clone, Copy)]
pub struct DisclosureLineInput<'a> {
    /// The CN chapter or heading this line is filed under.
    pub cn_category: &'a str,
    /// The Del. Reg. (EU) 2026/296 Art. 2 point letter claimed for this line.
    pub reason_point: char,
    /// Units discarded.
    pub units: u64,
    /// Weight discarded, in kilogrammes.
    pub weight_kg: u64,
    /// The treatment split, in the Annex I column order.
    pub preparing_for_reuse_pct: u8,
    pub recycling_pct: u8,
    pub other_recovery_pct: u8,
    pub disposal_pct: u8,
    pub unknown_pct: u8,
}

/// Borrowing view over the whole disclosure.
#[derive(Debug, Clone, Copy)]
pub struct UnsoldGoodsLintInput<'a> {
    /// Every line of the Annex I table.
    pub lines: &'a [DisclosureLineInput<'a>],
    /// Whether a consolidated disclosure listed its undertakings.
    pub consolidated_undertaking_count: Option<usize>,
    /// Annex I note (i) — measures taken, trimmed length.
    pub measures_taken_len: usize,
    /// Annex I note (j) — measures planned, trimmed length.
    pub measures_planned_len: usize,
}

/// A narrative row shorter than this is unlikely to describe a measure.
const MEANINGFUL_NARRATIVE_CHARS: usize = 20;

fn split_does_not_total_100(line: &DisclosureLineInput<'_>, index: usize) -> Option<LintFinding> {
    if disclosure::treatment_split_is_complete(
        line.preparing_for_reuse_pct,
        line.recycling_pct,
        line.other_recovery_pct,
        line.disposal_pct,
        line.unknown_pct,
    ) {
        return None;
    }
    let total = u16::from(line.preparing_for_reuse_pct)
        + u16::from(line.recycling_pct)
        + u16::from(line.other_recovery_pct)
        + u16::from(line.disposal_pct)
        + u16::from(line.unknown_pct);
    Some(LintFinding {
        code: "unsold_goods.treatment_split_does_not_total_100",
        field: "/lines",
        severity: LintSeverity::Warning,
        message: format!(
            "line {index}: waste treatment shares total {total}%, not 100% — Annex I note (i) \
             provides `unknown` for the share whose treatment could not be established, so no \
             share is left unaccounted for"
        ),
    })
}

fn cn_depth_too_shallow(line: &DisclosureLineInput<'_>, index: usize) -> Option<LintFinding> {
    if disclosure::cn_depth_is_correct(line.cn_category) {
        return None;
    }
    if line.cn_category.len() != 2 {
        return Some(LintFinding {
            code: "unsold_goods.cn_category_malformed",
            field: "/lines",
            severity: LintSeverity::Warning,
            message: format!(
                "line {index}: '{}' is not a CN chapter (2 digits) or heading (4 digits)",
                line.cn_category
            ),
        });
    }
    let headings = disclosure::annex_ii_headings_in_chapter(line.cn_category);
    Some(LintFinding {
        code: "unsold_goods.cn_category_needs_four_digits",
        field: "/lines",
        severity: LintSeverity::Warning,
        message: format!(
            "line {index}: chapter '{}' contains Annex II headings ({}), which Art. 3 requires be \
             disclosed at four digits — a chapter-level line hides which of them the goods were",
            line.cn_category,
            headings.join(", ")
        ),
    })
}

fn weight_without_units(line: &DisclosureLineInput<'_>, index: usize) -> Option<LintFinding> {
    if line.weight_kg > 0 && line.units == 0 {
        return Some(LintFinding {
            code: "unsold_goods.weight_without_units",
            field: "/lines",
            severity: LintSeverity::Notice,
            message: format!(
                "line {index}: {} kg discarded but zero units — note (f) allows the count to be \
                 estimated from the weight, so a figure is expected here",
                line.weight_kg
            ),
        });
    }
    None
}

/// Point (h) applies "only where none of the circumstances referred to in points
/// (a) to (g) are applicable", so claiming it for a category alongside a
/// stronger reason is a contradiction.
fn donation_claimed_alongside_stronger_reason(
    lines: &[DisclosureLineInput<'_>],
) -> Option<LintFinding> {
    let mut offending: Vec<&str> = Vec::new();
    for line in lines {
        if line.reason_point != 'h' {
            continue;
        }
        let same_category: Vec<char> = lines
            .iter()
            .filter(|l| l.cn_category == line.cn_category)
            .map(|l| l.reason_point)
            .collect();
        if !disclosure::donation_reason_is_admissible(&same_category)
            && !offending.contains(&line.cn_category)
        {
            offending.push(line.cn_category);
        }
    }
    if offending.is_empty() {
        return None;
    }
    Some(LintFinding {
        code: "unsold_goods.donation_reason_alongside_stronger_reason",
        field: "/lines",
        severity: LintSeverity::Warning,
        message: format!(
            "category/ies {} claim Art. 2 point (h) — offered for donation and not accepted — \
             alongside a point (a)-(g) reason. Point (h) is available only where none of (a) to \
             (g) applies",
            offending.join(", ")
        ),
    })
}

fn consolidated_without_undertakings(input: &UnsoldGoodsLintInput<'_>) -> Option<LintFinding> {
    if input.consolidated_undertaking_count == Some(0) {
        return Some(LintFinding {
            code: "unsold_goods.consolidated_disclosure_lists_no_undertakings",
            field: "/entity/scope",
            severity: LintSeverity::Warning,
            message:
                "a consolidated disclosure must list the subsidiaries or member undertakings it \
                 covers (Annex I note (c)); with none listed a reader cannot tell whose figures \
                 these are"
                    .into(),
        });
    }
    None
}

fn narrative_too_thin(len: usize, field: &'static str, note: char) -> Option<LintFinding> {
    if len >= MEANINGFUL_NARRATIVE_CHARS {
        return None;
    }
    Some(LintFinding {
        code: "unsold_goods.prevention_measures_not_described",
        field,
        severity: LintSeverity::Notice,
        message: format!(
            "Annex I note ({note}) asks for the measures themselves, not a placeholder — {len} \
             characters is unlikely to describe one"
        ),
    })
}

/// Run every unsold-goods disclosure lint.
#[must_use]
pub fn lint_unsold_goods(input: &UnsoldGoodsLintInput<'_>) -> Vec<LintFinding> {
    let mut findings = Vec::new();

    for (index, line) in input.lines.iter().enumerate() {
        findings.extend(split_does_not_total_100(line, index));
        findings.extend(cn_depth_too_shallow(line, index));
        findings.extend(weight_without_units(line, index));
    }
    findings.extend(donation_claimed_alongside_stronger_reason(input.lines));
    findings.extend(consolidated_without_undertakings(input));
    findings.extend(narrative_too_thin(
        input.measures_taken_len,
        "/measuresTaken",
        'i',
    ));
    findings.extend(narrative_too_thin(
        input.measures_planned_len,
        "/measuresPlanned",
        'j',
    ));

    findings
}
