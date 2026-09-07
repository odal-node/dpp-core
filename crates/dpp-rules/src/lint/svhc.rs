//! Substance-of-very-high-concern lints — cross-product-group, because REACH
//! Art. 33 is.
//!
//! # Why these are lints and not validation errors
//!
//! The candidate list this checks against is a **partial** snapshot: 32 entries
//! against an official 253 at the same revision. A check that incomplete cannot
//! be allowed to refuse a publish in either direction. It cannot say "this is
//! not an SVHC", because absence from the snapshot is absence of knowledge; and
//! it must not block a declaration it does recognise, because declaring a
//! substance is the correct behaviour the passport exists to record.
//!
//! What it can honestly do is tell the operator what their declaration implies
//! and what the check could not see. [`crate::lint`] is non-binding by design,
//! which is exactly the strength this check warrants.
//!
//! Every finding states the list revision it was produced against. A finding set
//! is only meaningful with respect to a specific list, and the reader has no
//! other way to learn which one ran.

use alloc::{format, vec::Vec};

use super::{LintFinding, LintSeverity};
use crate::chemicals::svhc::{
    CandidateList, SVHC_THRESHOLD_PCT, SvhcFindingKind, SvhcInput, check_svhc_declarations,
};

/// Classify `substances` against `list` and render the findings worth telling an
/// operator about.
///
/// Returns nothing for a declaration that is on the list below the Art. 33
/// threshold: that is a correctly recorded entry with no obligation attached and
/// no question to ask about it.
#[must_use]
pub fn lint_svhc_declarations(
    substances: &[SvhcInput<'_>],
    list: &CandidateList<'_>,
) -> Vec<LintFinding> {
    let mut findings = Vec::new();

    for f in check_svhc_declarations(substances, list) {
        match f.kind {
            // The obligation is REACH's, not this passport's, so this is a
            // notice about a duty the declaration implies — not a defect.
            SvhcFindingKind::MustDeclare => findings.push(LintFinding {
                code: "svhc.article_33_communication_owed",
                field: "svhcSubstances",
                severity: LintSeverity::Notice,
                message: format!(
                    "{} (CAS {}) is on the ECHA Candidate List as of {} and is declared at \
                     {:.3}% w/w, at or above the REACH Art. 33 threshold of {SVHC_THRESHOLD_PCT}% \
                     — is the downstream communication in place?",
                    f.substance_name, f.cas_number, list.as_of, f.concentration_pct
                ),
            }),
            // The one finding whose meaning depends on how complete the list is,
            // so it says so rather than leaving the reader to assume coverage.
            SvhcFindingKind::NotOnCheckedList => findings.push(LintFinding {
                code: "svhc.not_on_checked_list",
                field: "svhcSubstances",
                severity: LintSeverity::Notice,
                message: format!(
                    "{} (CAS {}) is not on the Candidate List this node checked against \
                     ({} of {} substances, as of {}) — it may still be an SVHC, or the CAS \
                     may be mistyped. Worth a manual check.",
                    f.substance_name,
                    f.cas_number,
                    list.entry_count(),
                    list.official_count,
                    list.as_of
                ),
            }),
            SvhcFindingKind::BelowThreshold => {}
        }
    }

    // Said once per passport rather than once per substance: it is a property of
    // the check, not of any entry. Emitted only when something was actually
    // checked — a passport declaring no substances has nothing to qualify.
    if !substances.is_empty() && !list.is_complete() {
        findings.push(LintFinding {
            code: "svhc.candidate_list_partial",
            field: "svhcSubstances",
            severity: LintSeverity::Notice,
            message: format!(
                "these declarations were checked against {} of the {} substances on the ECHA \
                 Candidate List as of {} — a clean result means nothing matched the substances \
                 this list knows about, not that no SVHCs are present.",
                list.entry_count(),
                list.official_count,
                list.as_of
            ),
        });
    }

    findings
}
