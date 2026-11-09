//! [`EvidenceKind`] — the documents the acts have an operator keep for a discard.

use serde::{Deserialize, Serialize};

/// A kind of document an economic operator keeps to stand behind a discard.
///
/// Three provisions name them, and they are different duties:
///
/// - **Del. Reg. (EU) 2026/296 Art. 3** — for each derogation relied on, the
///   documentation that substantiates it, kept for five years after the
///   destruction. Several points offer **alternatives** (a), (c), (f) and (g), and
///   one needs **two documents** (d), so which kinds a record lacks is not a
///   one-to-one lookup: see [`EvidenceRequirement`](super::EvidenceRequirement).
/// - **ESPR Art. 24(2)** and **Impl. Reg. (EU) 2026/2 Art. 4** — the documentation
///   demonstrating "the delivery and reception of the discarded products",
///   "including statements on the reception and treatment" of them, kept for five
///   years after the disclosure. The act's wording leaves open whether those
///   statements are ones the operator receives from a waste treatment operator or
///   concern products received back from one, so [`Self::TreatmentStatement`] says
///   neither.
/// - **Del. Reg. (EU) 2026/296 Art. 4** — a statement on the applicable derogation
///   the operator **gives** the waste treatment operator. That is a duty to provide,
///   not to keep, so [`Self::DerogationStatement`] is a kind a record may carry and
///   never one it is found lacking.
///
/// Art. 3's own list ends in "with the exception of when the information is
/// available to the competent national authority on the basis of another legal
/// act" — which this crate cannot know. A kind a record lacks is therefore one the
/// act lists, not proof of a breach.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum EvidenceKind {
    /// Art. 3(a)(i): a description of the health or safety concern, with the
    /// safety assessment under Regulation (EU) 2023/988 Arts. 6 to 8.
    SafetyAssessment,
    /// Art. 3(a)(ii): a test report showing non-compliant chemicals and naming the
    /// law that applies.
    ChemicalsTestReport,
    /// Art. 3(b): a self-assessment stating the type of non-compliance and the law.
    NonComplianceSelfAssessment,
    /// Art. 3(c): the final judicial decision or alternative dispute resolution
    /// decision that found the infringement.
    InfringementDecision,
    /// Art. 3(c): the notification of the infringement by a right holder, a
    /// competent authority or an entity authorised to act for a right holder.
    InfringementNotification,
    /// Art. 3(c): documentation of the operator's own internal investigation.
    InternalInvestigation,
    /// Art. 3(d): the licence, contract or agreement with the rightsholder that
    /// restricts distribution after a specified period.
    IpLicenceOrContract,
    /// Art. 3(d): the justification that destruction is appropriate and
    /// proportionate, which accompanies the licence.
    DestructionJustification,
    /// Art. 3(e): an inspection report or supporting documentation showing that
    /// the technical options for preparing for reuse or remanufacturing were
    /// assessed and found unfeasible.
    ReuseFeasibilityAssessment,
    /// Art. 3(f)(i): evidence of quality assessment procedures that prioritise
    /// restocking and repair.
    QualityAssessmentEvidence,
    /// Art. 3(f)(ii): an inspection record of the type and severity of the damage
    /// or defect and the unfeasibility of correcting it.
    InspectionRecord,
    /// Art. 3(g): evidence of the offer for donation.
    DonationOfferEvidence,
    /// Art. 3(h): a declaration that the product was received as a donation and no
    /// recipient could be found.
    DonationReceiptDeclaration,
    /// Art. 3(i): documentation that the product was received from a waste
    /// treatment operator and no recipient could be found.
    WasteOperatorSourceDocumentation,
    /// ESPR Art. 24(2), Impl. Reg. (EU) 2026/2 Art. 4: documentation of the
    /// delivery of the products to, and their reception by, the receiving operator.
    DeliveryDocumentation,
    /// Impl. Reg. (EU) 2026/2 Art. 4: a statement on the reception and treatment of
    /// the products.
    TreatmentStatement,
    /// Del. Reg. (EU) 2026/296 Art. 4: the statement on the applicable derogation
    /// given to the waste treatment operator. Never required of a record.
    DerogationStatement,
}
