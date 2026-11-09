//! What a record must be able to show, and what it lacks.

use crate::test_support::sample_discard_record;
use crate::unsold_goods::DiscardReason;
use crate::unsold_goods::{
    DiscardRecord, EvidenceKind, WasteTreatmentOperation, derogation_evidence,
};

use EvidenceKind::*;

/// A record whose only claim is that products were delivered somewhere.
fn bare_record() -> DiscardRecord {
    DiscardRecord {
        reason: None,
        treatment: WasteTreatmentOperation::Unknown,
        ..sample_discard_record()
    }
}

fn kinds(requirements: &[crate::unsold_goods::EvidenceRequirement]) -> Vec<Vec<EvidenceKind>> {
    requirements.iter().map(|r| r.any_of.clone()).collect()
}

/// ESPR Art. 24(2) reaches the delivery and reception of every discarded product,
/// not only those destroyed under a derogation.
#[test]
fn a_discard_with_no_derogation_still_needs_delivery_documentation() {
    let required = bare_record().required_evidence();
    assert_eq!(kinds(&required), [vec![DeliveryDocumentation]]);
}

#[test]
fn a_stated_treatment_needs_a_treatment_statement() {
    let record = DiscardRecord {
        treatment: WasteTreatmentOperation::Recycling,
        ..bare_record()
    };
    assert_eq!(
        kinds(&record.required_evidence()),
        [vec![DeliveryDocumentation], vec![TreatmentStatement]]
    );
}

/// An operator that could not establish the treatment has nothing for the
/// statement to confirm (Annex I note (i)).
#[test]
fn an_unknown_treatment_has_nothing_for_a_statement_to_confirm() {
    let required = bare_record().required_evidence();
    assert!(!kinds(&required).contains(&vec![TreatmentStatement]));
}

/// Art. 3(a): "either of the following" — a safety assessment or a chemicals test
/// report.
#[test]
fn point_a_is_met_by_either_alternative() {
    let record = DiscardRecord {
        reason: Some(DiscardReason::DangerousProduct),
        ..sample_discard_record()
    };
    let base = [DeliveryDocumentation, TreatmentStatement];

    assert_eq!(record.missing_evidence(&base).len(), 1);
    for alternative in [SafetyAssessment, ChemicalsTestReport] {
        let held = [base[0], base[1], alternative];
        assert!(record.missing_evidence(&held).is_empty(), "{alternative:?}");
    }
}

/// Art. 3(c): the decision, the notification, or the operator's own investigation.
#[test]
fn point_c_is_met_by_any_of_three() {
    let record = DiscardRecord {
        reason: Some(DiscardReason::IntellectualPropertyInfringement),
        ..sample_discard_record()
    };
    for alternative in [
        InfringementDecision,
        InfringementNotification,
        InternalInvestigation,
    ] {
        let held = [DeliveryDocumentation, TreatmentStatement, alternative];
        assert!(record.missing_evidence(&held).is_empty(), "{alternative:?}");
    }
}

/// Art. 3(d): a licence, contract or agreement "accompanied by a justification" —
/// two documents, not a choice between them.
#[test]
fn point_d_needs_the_contract_and_the_justification_not_either() {
    let record = DiscardRecord {
        reason: Some(DiscardReason::LicensedPeriodExpired),
        ..sample_discard_record()
    };
    let base = [DeliveryDocumentation, TreatmentStatement];

    let only_contract = [base[0], base[1], IpLicenceOrContract];
    assert_eq!(
        kinds(&record.missing_evidence(&only_contract)),
        [vec![DestructionJustification]]
    );

    let only_justification = [base[0], base[1], DestructionJustification];
    assert_eq!(
        kinds(&record.missing_evidence(&only_justification)),
        [vec![IpLicenceOrContract]]
    );

    let both = [
        base[0],
        base[1],
        IpLicenceOrContract,
        DestructionJustification,
    ];
    assert!(record.missing_evidence(&both).is_empty());
}

/// Art. 3(f) serves both Art. 2 point (f) and point (g): the quality-assessment
/// evidence or an inspection record.
#[test]
fn points_f_and_g_share_one_requirement_met_by_either_alternative() {
    let f = derogation_evidence(DiscardReason::DamagedOrContaminated);
    let g = derogation_evidence(DiscardReason::DefectiveBeyondRepair);

    assert_eq!(f, g);
    assert_eq!(
        kinds(&f),
        [vec![QualityAssessmentEvidence, InspectionRecord]]
    );
    assert!(f[0].basis.ends_with("Art. 3(f)"));
}

/// 🚨 Art. 3 runs one letter behind Art. 2 from point (h): Art. 3(g) is for Art. 2
/// point (h), 3(h) for (i) and 3(i) for (j). Citing Art. 2's letter would send a
/// reader to the wrong paragraph.
#[test]
fn art_3_letters_after_f_run_one_behind_art_2() {
    let basis = |reason| derogation_evidence(reason)[0].basis;

    assert!(basis(DiscardReason::OfferedForDonationNotAccepted).ends_with("Art. 3(g)"));
    assert!(basis(DiscardReason::DonatedButNoRecipientFound).ends_with("Art. 3(h)"));
    assert!(basis(DiscardReason::ReusedButNoRecipientFound).ends_with("Art. 3(i)"));
}

#[test]
fn points_b_e_h_i_and_j_each_name_one_document() {
    let expected = [
        (
            DiscardReason::NonCompliantWithLaw,
            NonComplianceSelfAssessment,
        ),
        (
            DiscardReason::MarkingsCannotBeRemoved,
            ReuseFeasibilityAssessment,
        ),
        (
            DiscardReason::OfferedForDonationNotAccepted,
            DonationOfferEvidence,
        ),
        (
            DiscardReason::DonatedButNoRecipientFound,
            DonationReceiptDeclaration,
        ),
        (
            DiscardReason::ReusedButNoRecipientFound,
            WasteOperatorSourceDocumentation,
        ),
    ];
    for (reason, kind) in expected {
        assert_eq!(
            kinds(&derogation_evidence(reason)),
            [vec![kind]],
            "{reason:?}"
        );
    }
}

#[test]
fn every_derogation_has_documentation_cited_to_art_3() {
    for reason in DiscardReason::ALL {
        let required = derogation_evidence(*reason);
        assert!(!required.is_empty(), "{reason:?}");
        for requirement in required {
            assert!(!requirement.any_of.is_empty(), "{reason:?}");
            assert!(
                requirement
                    .basis
                    .starts_with("Del. Reg. (EU) 2026/296 Art. 3"),
                "{reason:?}: {}",
                requirement.basis
            );
        }
    }
}

/// Art. 4 has the operator give the waste treatment operator a statement; it is a
/// duty to provide, not to keep, so a record is never found lacking one.
#[test]
fn the_statement_to_the_waste_operator_is_never_required() {
    let mut records = vec![bare_record(), sample_discard_record()];
    for reason in DiscardReason::ALL {
        records.push(DiscardRecord {
            reason: Some(*reason),
            ..sample_discard_record()
        });
    }
    for record in records {
        let required = record.required_evidence();
        assert!(
            required
                .iter()
                .all(|r| !r.any_of.contains(&DerogationStatement)),
            "{record:?}"
        );
    }
}

#[test]
fn an_unrelated_document_does_not_meet_a_requirement() {
    let record = DiscardRecord {
        reason: Some(DiscardReason::DangerousProduct),
        ..sample_discard_record()
    };
    let held = [DeliveryDocumentation, TreatmentStatement, InspectionRecord];
    assert_eq!(
        kinds(&record.missing_evidence(&held)),
        [vec![SafetyAssessment, ChemicalsTestReport]]
    );
}

/// With nothing held, every requirement is missing and in the order they are
/// required.
#[test]
fn a_record_holding_nothing_lacks_everything_it_requires() {
    let record = sample_discard_record();
    assert_eq!(record.missing_evidence(&[]), record.required_evidence());
}
