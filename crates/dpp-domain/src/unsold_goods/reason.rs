//! [`DiscardReason`] — the closed list of circumstances under which an unsold
//! consumer product may lawfully be destroyed.

use serde::{Deserialize, Serialize};

/// Why a line of unsold consumer products was discarded.
///
/// # This list is the law's, not ours
///
/// **Annex I note (h) of Commission Implementing Regulation (EU) 2026/2:**
/// reasons for discarding "shall, where applicable, refer to the reasons listed
/// in delegated acts adopted pursuant to Article 25(5) of Regulation (EU)
/// 2024/1781". That delegated act is **Commission Delegated Regulation (EU)
/// 2026/296**, and its **Article 2** enumerates the derogations from the
/// destruction prohibition, points (a) to (j). The variants below are those
/// points, in that order.
///
/// The previous model carried an invented set — `EndOfSeason`, `QualityDefect`,
/// `PackagingDefect`, `OverProduction`, `CustomerReturn`, `Other` — none of
/// which appears in the Regulation. Two of them (`EndOfSeason`,
/// `OverProduction`) name commercial circumstances that are *not* derogations at
/// all, so a disclosure using them asserted a lawful destruction that the act
/// does not permit.
///
/// # Two constraints that are not in the type
///
/// **Point (h) is subordinate.** It applies "only where none of the
/// circumstances referred to in points (a) to (g) are applicable" — a condition
/// over the whole set of available reasons, which a single enum value cannot
/// express. It is checked in `dpp-rules`.
///
/// **Every reason carries a documentation duty.** Art. 3 of the same act
/// requires the operator to keep specified evidence per derogation for **five
/// years** after destruction, in electronic form, produced within 30 days of a
/// competent authority's request. That evidence is not passport data and is not
/// modelled here; the reason recorded is the claim, not the proof of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum DiscardReason {
    /// **(a)** A dangerous product within the meaning of Regulation (EU) 2023/988.
    DangerousProduct,
    /// **(b)** Unfit for purpose because non-compliant with Union or national
    /// law, for reasons other than (a), where destruction is required by law or
    /// is the appropriate and proportionate corrective action.
    NonCompliantWithLaw,
    /// **(c)** Found to infringe intellectual property rights by judicial or ADR
    /// decision, rightsholder notification, or a substantiated internal
    /// investigation.
    IntellectualPropertyInfringement,
    /// **(d)** Subject to an IP-protecting licence whose permitted period for
    /// sale or transfer has expired.
    LicensedPeriodExpired,
    /// **(e)** Unsuitable for preparing for reuse or remanufacturing because
    /// protected or inappropriate labels, logos or design characteristics cannot
    /// technically be removed or rendered inaccessible.
    MarkingsCannotBeRemoved,
    /// **(f)** Reasonably unacceptable for consumer use through damage,
    /// deterioration or contamination, where repair and refurbishment are not
    /// technically feasible or cost-effective.
    DamagedOrContaminated,
    /// **(g)** Unfit for its intended purpose through a design or manufacturing
    /// defect for which repair is not technically feasible.
    DefectiveBeyondRepair,
    /// **(h)** Offered for donation — to at least three suitable social economy
    /// entities in the Union, or on an easily accessible page of the operator's
    /// website, for at least eight weeks — and not accepted.
    ///
    /// Available **only** where none of (a) to (g) applies.
    OfferedForDonationNotAccepted,
    /// **(i)** Received as a donation by a social economy entity in the Union,
    /// but no recipient could be found.
    DonatedButNoRecipientFound,
    /// **(j)** Made available on the market after being prepared for reuse by a
    /// waste treatment operator, but no recipient could be found.
    ReusedButNoRecipientFound,
}

impl DiscardReason {
    /// Every reason, in the order Del. Reg. (EU) 2026/296 Art. 2 lists them.
    pub const ALL: &'static [Self] = &[
        Self::DangerousProduct,
        Self::NonCompliantWithLaw,
        Self::IntellectualPropertyInfringement,
        Self::LicensedPeriodExpired,
        Self::MarkingsCannotBeRemoved,
        Self::DamagedOrContaminated,
        Self::DefectiveBeyondRepair,
        Self::OfferedForDonationNotAccepted,
        Self::DonatedButNoRecipientFound,
        Self::ReusedButNoRecipientFound,
    ];

    /// The point of Del. Reg. (EU) 2026/296 Art. 2 this reason is.
    ///
    /// Kept so a disclosure, a determination or an error message can cite the
    /// act rather than our own name for it.
    #[must_use]
    pub fn article_2_point(self) -> char {
        match self {
            Self::DangerousProduct => 'a',
            Self::NonCompliantWithLaw => 'b',
            Self::IntellectualPropertyInfringement => 'c',
            Self::LicensedPeriodExpired => 'd',
            Self::MarkingsCannotBeRemoved => 'e',
            Self::DamagedOrContaminated => 'f',
            Self::DefectiveBeyondRepair => 'g',
            Self::OfferedForDonationNotAccepted => 'h',
            Self::DonatedButNoRecipientFound => 'i',
            Self::ReusedButNoRecipientFound => 'j',
        }
    }

    /// Whether this reason is available only when no other applies — true for
    /// point (h) alone, whose text begins "only where none of the circumstances
    /// referred to in points (a) to (g) are applicable".
    #[must_use]
    pub fn is_subordinate(self) -> bool {
        matches!(self, Self::OfferedForDonationNotAccepted)
    }
}
