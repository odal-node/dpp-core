//! [`EvidenceRequirement`] — one thing a record must be able to show.

use serde::Serialize;

use super::kind::EvidenceKind;
use crate::unsold_goods::DiscardReason;

/// One requirement: a document of **any one** of these kinds.
///
/// A set of requirements is read as an AND of ORs, which is the shape of
/// Del. Reg. (EU) 2026/296 Art. 3: point (a) is "either of the following", point (c)
/// any of a decision, a notification or an investigation, and point (d) a licence
/// **and** a justification — two requirements, each satisfied by its own kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceRequirement {
    /// Any one of these kinds meets the requirement.
    pub any_of: Vec<EvidenceKind>,
    /// The provision that makes it a requirement, for a message or an export.
    pub basis: &'static str,
}

impl EvidenceRequirement {
    /// Whether a document of one of the held kinds meets this requirement.
    #[must_use]
    pub fn is_met_by(&self, held: &[EvidenceKind]) -> bool {
        self.any_of.iter().any(|kind| held.contains(kind))
    }
}

/// The documentation Del. Reg. (EU) 2026/296 Art. 3 requires for a derogation.
///
/// 🚨 Art. 3 does not use Art. 2's letters after (f). Art. 3(f) covers both Art. 2
/// points (f) and (g); Art. 3(g) is for Art. 2 point (h), 3(h) for (i) and 3(i) for
/// (j). The [`EvidenceRequirement::basis`] cites Art. 3's own letter.
#[must_use]
pub fn derogation_evidence(reason: DiscardReason) -> Vec<EvidenceRequirement> {
    use EvidenceKind::*;

    fn one_of(any_of: &[EvidenceKind], basis: &'static str) -> EvidenceRequirement {
        EvidenceRequirement {
            any_of: any_of.to_vec(),
            basis,
        }
    }

    match reason {
        DiscardReason::DangerousProduct => vec![one_of(
            &[SafetyAssessment, ChemicalsTestReport],
            "Del. Reg. (EU) 2026/296 Art. 3(a)",
        )],
        DiscardReason::NonCompliantWithLaw => vec![one_of(
            &[NonComplianceSelfAssessment],
            "Del. Reg. (EU) 2026/296 Art. 3(b)",
        )],
        DiscardReason::IntellectualPropertyInfringement => vec![one_of(
            &[
                InfringementDecision,
                InfringementNotification,
                InternalInvestigation,
            ],
            "Del. Reg. (EU) 2026/296 Art. 3(c)",
        )],
        DiscardReason::LicensedPeriodExpired => vec![
            one_of(&[IpLicenceOrContract], "Del. Reg. (EU) 2026/296 Art. 3(d)"),
            one_of(
                &[DestructionJustification],
                "Del. Reg. (EU) 2026/296 Art. 3(d)",
            ),
        ],
        DiscardReason::MarkingsCannotBeRemoved => vec![one_of(
            &[ReuseFeasibilityAssessment],
            "Del. Reg. (EU) 2026/296 Art. 3(e)",
        )],
        DiscardReason::DamagedOrContaminated | DiscardReason::DefectiveBeyondRepair => {
            vec![one_of(
                &[QualityAssessmentEvidence, InspectionRecord],
                "Del. Reg. (EU) 2026/296 Art. 3(f)",
            )]
        }
        DiscardReason::OfferedForDonationNotAccepted => vec![one_of(
            &[DonationOfferEvidence],
            "Del. Reg. (EU) 2026/296 Art. 3(g)",
        )],
        DiscardReason::DonatedButNoRecipientFound => vec![one_of(
            &[DonationReceiptDeclaration],
            "Del. Reg. (EU) 2026/296 Art. 3(h)",
        )],
        DiscardReason::ReusedButNoRecipientFound => vec![one_of(
            &[WasteOperatorSourceDocumentation],
            "Del. Reg. (EU) 2026/296 Art. 3(i)",
        )],
    }
}
