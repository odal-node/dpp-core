//! Discard-record lints — consistency checks across the fields of one discard
//! of unsold consumer products.
//!
//! Advisory only, like the disclosure lints beside them. A record that trips one
//! is still a record: what was discarded has to be kept either way, and a finding
//! says only that a reader who knows the acts would look twice.

use alloc::{format, vec::Vec};

use super::super::{LintFinding, LintSeverity};
use crate::unsold_goods::annex_vii::is_within_annex_vii_scope;
use crate::unsold_goods::disclosure::category_of_cn_code;

/// Borrowing view over one discard record.
#[derive(Debug, Clone, Copy)]
pub struct DiscardRecordLintInput<'a> {
    /// The product's own CN codes — more than one only for products sold
    /// together (Annex I note (f)).
    pub cn_codes: &'a [&'a str],
    /// The Del. Reg. (EU) 2026/296 Art. 2 point letter relied on, if any.
    pub reason_point: Option<char>,
    /// Whether the discard was made for the sole purpose of delivering the
    /// products for preparing for reuse.
    pub prepared_for_reuse: bool,
    /// Weight discarded, in grams.
    pub weight_grams: u64,
}

/// Whether the record names at least one CN code and every code is readable.
/// The Annex VII checks below say nothing about a record they cannot place.
fn cn_codes_readable(input: &DiscardRecordLintInput<'_>) -> bool {
    !input.cn_codes.is_empty()
        && input
            .cn_codes
            .iter()
            .all(|code| category_of_cn_code(code).is_some())
}

/// A record names no CN code, or one that is not 6, 8 or 10 digits.
fn cn_code_unreadable(input: &DiscardRecordLintInput<'_>) -> Option<LintFinding> {
    if cn_codes_readable(input) {
        return None;
    }
    Some(LintFinding {
        code: "unsold_goods.record_cn_code_unreadable",
        field: "/cnCodes",
        severity: LintSeverity::Warning,
        message: "the record carries no CN code, or one that is not 6, 8 or 10 digits — Impl. \
                  Reg. (EU) 2026/2 Art. 3 files a discard under the first two (or, for Annex II \
                  products, four) digits of its CN code, so a record without a readable one \
                  cannot be counted in the disclosure"
            .into(),
    })
}

/// A record that weighs nothing cannot be apportioned across treatments.
fn without_weight(input: &DiscardRecordLintInput<'_>) -> Option<LintFinding> {
    if input.weight_grams > 0 {
        return None;
    }
    Some(LintFinding {
        code: "unsold_goods.record_without_weight",
        field: "/weight",
        severity: LintSeverity::Warning,
        message: "the record weighs nothing — Annex I note (g) requires the weight, which may be \
                  estimated from an accurately counted number of units (and marked as an \
                  estimate), so a figure is expected; a record without one cannot be apportioned \
                  across waste treatments and is left out of the disclosure"
            .into(),
    })
}

/// ESPR Art. 25(1) prohibits destroying Annex VII products, and Del. Reg. (EU)
/// 2026/296 Art. 2 permits it only under one of its points. Art. 2(34) takes
/// discarding "for the sole purpose of delivering the discarded product for
/// preparing for reuse" out of destruction, so that discard needs no point.
fn annex_vii_discard_without_derogation(input: &DiscardRecordLintInput<'_>) -> Option<LintFinding> {
    let in_annex_vii = input.cn_codes.iter().any(|c| is_within_annex_vii_scope(c));
    if !cn_codes_readable(input)
        || !in_annex_vii
        || input.prepared_for_reuse
        || input.reason_point.is_some()
    {
        return None;
    }
    Some(LintFinding {
        code: "unsold_goods.annex_vii_discard_without_derogation",
        field: "/reason",
        severity: LintSeverity::Warning,
        message: "this discards an Annex VII product (apparel, clothing accessories or footwear) \
                  with no derogation recorded — ESPR Art. 25(1) prohibits destroying it, and Del. \
                  Reg. (EU) 2026/296 Art. 2 permits that only under one of its points (a) to (j), \
                  with the Art. 3 documentation to hand. If the products went for preparing for \
                  reuse, the treatment should say so"
            .into(),
    })
}

/// The derogations are exceptions to the Art. 25 prohibition, so they have
/// nothing to except for a good outside Annex VII or a discard that is not a
/// destruction. Claiming one is not wrong, only worth a second look: Art. 24(1)(b)
/// asks for "the relevant derogation" of what was discarded.
fn derogation_without_annex_vii_destruction(
    input: &DiscardRecordLintInput<'_>,
) -> Option<LintFinding> {
    let point = input.reason_point?;
    let in_annex_vii = input.cn_codes.iter().any(|c| is_within_annex_vii_scope(c));
    if !cn_codes_readable(input) || (in_annex_vii && !input.prepared_for_reuse) {
        return None;
    }
    let why = if input.prepared_for_reuse {
        "discarding for the sole purpose of preparing for reuse is not destruction (ESPR Art. \
         2(34))"
    } else {
        "no CN code on the record is in Annex VII, whose products are the only ones whose \
         destruction Art. 25(1) prohibits"
    };
    Some(LintFinding {
        code: "unsold_goods.derogation_without_annex_vii_destruction",
        field: "/reason",
        severity: LintSeverity::Notice,
        message: format!(
            "point ({point}) of Del. Reg. (EU) 2026/296 Art. 2 is recorded, but the derogations \
             except an Annex VII destruction from the prohibition, and {why} — is the point \
             intended?"
        ),
    })
}

/// Run every discard-record lint.
#[must_use]
pub fn lint_discard_record(input: &DiscardRecordLintInput<'_>) -> Vec<LintFinding> {
    let mut findings = Vec::new();
    findings.extend(cn_code_unreadable(input));
    findings.extend(without_weight(input));
    findings.extend(annex_vii_discard_without_derogation(input));
    findings.extend(derogation_without_annex_vii_destruction(input));
    findings
}
