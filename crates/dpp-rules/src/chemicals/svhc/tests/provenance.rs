//! Tests of the `svhc` module: provenance.

use super::super::*;

#[test]
fn the_embedded_list_reports_its_own_size_and_revision() {
    let list = CandidateList::embedded();
    assert_eq!(list.entry_count(), ECHA_CANDIDATE_LIST.len());
    assert_eq!(list.as_of, "2026-02-04");
    assert_eq!(list.official_count, ECHA_CANDIDATE_LIST_OFFICIAL_COUNT);
}

#[test]
fn snapshot_is_known_incomplete() {
    // The point of this type: a caller must be able to see that a clean
    // finding set was produced from a partial list. If this ever becomes
    // true, the coverage gap has genuinely been closed and the docs saying
    // otherwise need updating with it.
    assert!(
        !CandidateList::embedded().is_complete(),
        "snapshot now claims completeness — update the module docs and the \
             adjacent-obligations research note if that is genuinely true"
    );
}

#[test]
fn a_supplied_list_is_checked_instead_of_the_embedded_one() {
    // The whole point of the parameter: a substance the embedded snapshot
    // has never heard of is recognised when a newer list carries it.
    const NEWER: &[&str] = &["7440-02-0"];
    let list = CandidateList {
        cas_numbers: NEWER,
        as_of: "2027-01-01",
        official_count: 1,
    };
    let nickel = SvhcInput {
        cas_number: "7440-02-0",
        substance_name: "Nickel",
        concentration_pct: 0.5,
    };

    assert_eq!(
        check_svhc_declarations_embedded(&[nickel])[0].kind,
        SvhcFindingKind::NotOnCheckedList,
        "the embedded snapshot does not carry this CAS"
    );
    assert_eq!(
        check_svhc_declarations(&[nickel], &list)[0].kind,
        SvhcFindingKind::MustDeclare,
        "the supplied list does, so the same declaration classifies differently"
    );
}

#[test]
fn the_2026_02_04_additions_are_present() {
    assert!(
        ECHA_CANDIDATE_LIST.contains(&"110-54-3"),
        "n-Hexane missing"
    );
    assert!(ECHA_CANDIDATE_LIST.contains(&"1478-61-1"), "BPAF missing");
}

#[test]
fn embedded_list_has_no_duplicates() {
    for (i, cas) in ECHA_CANDIDATE_LIST.iter().enumerate() {
        assert!(
            !ECHA_CANDIDATE_LIST[..i].contains(cas),
            "duplicate CAS {cas} in the embedded candidate list"
        );
    }
}

#[test]
fn absent_cas_is_unknown_not_cleared() {
    // A real SVHC we do not embed must not read as "fine". This guards the
    // semantics of NotOnCheckedList against a future well-meaning rename.
    let unknown = SvhcInput {
        cas_number: "50-00-0", // formaldehyde — on the official list, not embedded
        substance_name: "Formaldehyde",
        concentration_pct: 5.0,
    };
    let f = check_svhc_declarations_embedded(&[unknown]);
    assert_eq!(f[0].kind, SvhcFindingKind::NotOnCheckedList);
}
