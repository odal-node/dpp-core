//! Tests of the `svhc` module.

use super::super::*;

fn bpa(pct: f64) -> SvhcInput<'static> {
    SvhcInput {
        cas_number: "80-05-7",
        substance_name: "Bisphenol A",
        concentration_pct: pct,
    }
}

// ── Structural validation ────────────────────────────────────────────────

#[test]
fn svhc_valid_and_invalid() {
    assert!(validate_svhc_substances(&[bpa(0.15)]).is_ok());
    assert!(validate_svhc_substances(&[]).is_ok()); // empty = checked, none found

    let empty_cas = SvhcInput {
        cas_number: "",
        substance_name: "x",
        concentration_pct: 0.5,
    };
    assert!(validate_svhc_substances(&[empty_cas]).is_err());

    let bad_conc = SvhcInput {
        cas_number: "80-05-7",
        substance_name: "x",
        concentration_pct: -1.0,
    };
    assert!(validate_svhc_substances(&[bad_conc]).is_err());
}

#[test]
fn nan_concentration_rejected() {
    assert!(validate_svhc_substances(&[bpa(f64::NAN)]).is_err());
}

#[test]
fn infinity_concentration_rejected() {
    assert!(validate_svhc_substances(&[bpa(f64::INFINITY)]).is_err());
}

#[test]
fn malformed_cas_rejected_by_structural_validator() {
    // 80-05-8 has a wrong check digit (correct is 7).
    let bad = SvhcInput {
        cas_number: "80-05-8",
        substance_name: "BPA wrong CAS",
        concentration_pct: 0.5,
    };
    let err = validate_svhc_substances(&[bad]).unwrap_err();
    assert!(err.contains("check digit"), "unexpected: {err}");
}

#[test]
fn wrong_cas_format_rejected() {
    let bad = SvhcInput {
        cas_number: "NOTACAS",
        substance_name: "X",
        concentration_pct: 0.1,
    };
    assert!(validate_svhc_substances(&[bad]).is_err());
}

// ── Candidate list + threshold semantics ─────────────────────────────────

#[test]
fn bpa_above_threshold_is_must_declare() {
    let findings = check_svhc_declarations_embedded(&[bpa(0.15)]);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].kind, SvhcFindingKind::MustDeclare);
}

#[test]
fn bpa_below_threshold_is_informational() {
    let findings = check_svhc_declarations_embedded(&[bpa(0.05)]);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].kind, SvhcFindingKind::BelowThreshold);
}

#[test]
fn bpa_exactly_at_threshold_is_must_declare() {
    // Boundary: >= 0.1 triggers obligation.
    let findings = check_svhc_declarations_embedded(&[bpa(SVHC_THRESHOLD_PCT)]);
    assert_eq!(findings[0].kind, SvhcFindingKind::MustDeclare);
}

#[test]
fn unknown_cas_flagged_as_not_in_embedded_list() {
    // 7440-66-6 = Zinc — valid CAS format, not an SVHC.
    let zinc = SvhcInput {
        cas_number: "7440-66-6",
        substance_name: "Zinc",
        concentration_pct: 5.0,
    };
    let findings = check_svhc_declarations_embedded(&[zinc]);
    assert_eq!(findings[0].kind, SvhcFindingKind::NotOnCheckedList);
}

#[test]
fn mixed_declarations_produce_correct_findings() {
    let above = bpa(0.2);
    let below = SvhcInput {
        cas_number: "84-74-2",
        substance_name: "DBP",
        concentration_pct: 0.05,
    };
    let unknown = SvhcInput {
        cas_number: "7440-66-6",
        substance_name: "Zinc",
        concentration_pct: 5.0,
    };
    let findings = check_svhc_declarations_embedded(&[above, below, unknown]);
    assert_eq!(findings[0].kind, SvhcFindingKind::MustDeclare);
    assert_eq!(findings[1].kind, SvhcFindingKind::BelowThreshold);
    assert_eq!(findings[2].kind, SvhcFindingKind::NotOnCheckedList);
    // Indices are preserved
    assert_eq!(findings[0].index, 0);
    assert_eq!(findings[1].index, 1);
    assert_eq!(findings[2].index, 2);
}

#[test]
fn threshold_constant_is_0_1_pct() {
    assert!(
        (SVHC_THRESHOLD_PCT - 0.1).abs() < f64::EPSILON,
        "REACH Art. 33 threshold must be exactly 0.1 % w/w"
    );
}

#[test]
fn bpa_on_candidate_list() {
    assert!(
        ECHA_CANDIDATE_LIST.contains(&"80-05-7"),
        "BPA (80-05-7) must be on the embedded candidate list"
    );
}

#[test]
fn all_embedded_cas_numbers_have_valid_format() {
    // Verifies that no data-entry error was made when compiling the candidate list.
    for &cas in ECHA_CANDIDATE_LIST {
        let s = [SvhcInput {
            cas_number: cas,
            substance_name: "test",
            concentration_pct: 0.5,
        }];
        assert!(
            validate_svhc_substances(&s).is_ok(),
            "embedded candidate list entry fails CAS format check: {cas}"
        );
    }
}
