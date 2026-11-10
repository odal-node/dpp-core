//! [`WasteTreatmentSplit`] — where a discarded line actually went, as
//! percentages of weight.

use serde::{Deserialize, Serialize};

/// How a discarded product line was treated, split across the operations
/// Annex I of Commission Implementing Regulation (EU) 2026/2 names.
///
/// # Percentages of weight, not of units
///
/// **Annex I note (i):** "The percentages of disclosed waste treatment
/// operations shall be calculated on the basis of the **weight** of discarded
/// unsold consumer products." The unit count on the line plays no part in this
/// split.
///
/// The same note requires the information to be "retrieved from waste treatment
/// operators that collect unsold consumer products", and where it cannot be
/// obtained, treatment "shall be listed as `unknown`". [`Self::unknown_pct`] is
/// therefore a real answer, not a missing one — it says the operator asked and
/// could not find out, which is a different statement from an incomplete
/// disclosure.
///
/// # Total destruction is derived, never stored
///
/// Annex I prints a "Total destruction (in %)" column, and note (i) defines it:
/// "**Destruction is the sum of recycling, other recovery and disposal.**" It is
/// therefore computed by [`Self::total_destruction_pct`] and has no field.
/// Storing it would create a second home for a number the act already defines in
/// terms of three others, and the two could disagree.
///
/// Note what that definition puts *outside* destruction: preparing for reuse,
/// and unknown. Recycling is inside it. That is the act's arithmetic and it is
/// not the intuitive one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WasteTreatmentSplit {
    /// Preparing for reuse, as defined in Directive 2008/98/EC Art. 3(16).
    pub preparing_for_reuse_pct: u8,
    /// Recycling, as defined in Directive 2008/98/EC Art. 3(17).
    pub recycling_pct: u8,
    /// Other recovery — e.g. energy recovery — per Directive 2008/98/EC Art. 3(15).
    pub other_recovery_pct: u8,
    /// Disposal, as defined in Directive 2008/98/EC Art. 3(19).
    pub disposal_pct: u8,
    /// The share whose treatment the operator could not establish. Annex I note
    /// (i) provides for this explicitly.
    pub unknown_pct: u8,
}

impl WasteTreatmentSplit {
    /// Total destruction: **recycling + other recovery + disposal**, per Annex I
    /// note (i).
    ///
    /// Returns `u16` rather than `u8` deliberately — three `u8` shares can sum
    /// past 255 in a malformed record, and a panic or a silent wrap is a worse
    /// answer than a number the caller can see is impossible.
    #[must_use]
    pub fn total_destruction_pct(&self) -> u16 {
        u16::from(self.recycling_pct)
            + u16::from(self.other_recovery_pct)
            + u16::from(self.disposal_pct)
    }

    /// The sum of every share, which a well-formed split makes 100.
    ///
    /// Not asserted here: this type describes what was disclosed, and refusing
    /// to represent a disclosure that does not add up would make an invalid
    /// record unreadable rather than reportable. The check belongs with the
    /// other cross-field rules in `dpp-rules`.
    #[must_use]
    pub fn total_pct(&self) -> u16 {
        u16::from(self.preparing_for_reuse_pct)
            + u16::from(self.recycling_pct)
            + u16::from(self.other_recovery_pct)
            + u16::from(self.disposal_pct)
            + u16::from(self.unknown_pct)
    }
}
