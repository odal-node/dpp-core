//! What the [SVHC lints](super::svhc) report, and what they stay silent about.

use alloc::vec;
use alloc::vec::Vec;

use super::svhc::lint_svhc_declarations;
use crate::chemicals::svhc::{CandidateList, SvhcInput};

fn input<'a>(cas: &'a str, name: &'a str, pct: f64) -> SvhcInput<'a> {
    SvhcInput {
        cas_number: cas,
        substance_name: name,
        concentration_pct: pct,
    }
}

/// A two-entry list that claims to be complete, so the partial-coverage notice
/// stays out of the way of the per-substance assertions.
fn complete_list() -> CandidateList<'static> {
    const CAS: &[&str] = &["80-05-7", "117-81-7"];
    CandidateList {
        cas_numbers: CAS,
        as_of: "2026-02-04",
        official_count: 2,
    }
}

#[test]
fn an_entry_at_or_above_the_threshold_raises_the_article_33_notice() {
    let subs = vec![input("80-05-7", "Bisphenol A", 0.1)];
    let findings = lint_svhc_declarations(&subs, &complete_list());
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].code, "svhc.article_33_communication_owed");
    assert!(
        findings[0].message.contains("2026-02-04"),
        "a finding must say which list revision produced it: {}",
        findings[0].message
    );
}

#[test]
fn an_entry_below_the_threshold_says_nothing() {
    let subs = vec![input("80-05-7", "Bisphenol A", 0.05)];
    assert!(lint_svhc_declarations(&subs, &complete_list()).is_empty());
}

#[test]
fn an_unknown_cas_is_a_question_never_a_clearance() {
    let subs = vec![input("7440-43-9", "Cadmium", 5.0)];
    let findings = lint_svhc_declarations(&subs, &complete_list());
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].code, "svhc.not_on_checked_list");
    assert!(
        findings[0].message.contains("may still be an SVHC"),
        "absence from the list must never read as absence of the hazard"
    );
}

#[test]
fn a_partial_list_qualifies_its_own_result_once() {
    let subs = vec![
        input("80-05-7", "Bisphenol A", 0.05),
        input("117-81-7", "DEHP", 0.05),
    ];
    let partial = CandidateList {
        official_count: 253,
        ..complete_list()
    };
    let findings = lint_svhc_declarations(&subs, &partial);
    // Both entries are below threshold, so the only finding is the caveat — and
    // it appears once, not once per substance.
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].code, "svhc.candidate_list_partial");
}

#[test]
fn no_declarations_means_nothing_to_qualify() {
    let partial = CandidateList {
        official_count: 253,
        ..complete_list()
    };
    assert!(lint_svhc_declarations(&[], &partial).is_empty());
}

#[test]
fn the_embedded_baseline_is_partial_and_says_so() {
    let list = CandidateList::embedded();
    assert!(
        !list.is_complete(),
        "the embedded snapshot is a subset; if this ever passes, the \
         partial-coverage lint stops firing and its message needs rewriting"
    );
    let subs = vec![input("80-05-7", "Bisphenol A", 0.5)];
    let codes: Vec<&str> = lint_svhc_declarations(&subs, &list)
        .iter()
        .map(|f| f.code)
        .collect();
    assert!(codes.contains(&"svhc.article_33_communication_owed"));
    assert!(codes.contains(&"svhc.candidate_list_partial"));
}
