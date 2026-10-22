//! [`DueDiligenceSecondLife`] — whether Art. 47's second paragraph reaches a
//! battery.

/// Whether Art. 47's second paragraph reaches a battery.
///
/// Restated here rather than shared with
/// [`Art8SecondLife`](crate::batteries::recycled_content::Art8SecondLife),
/// although the two paragraphs use the same words today. Two articles that agree
/// are two articles that can be amended separately, and a shared type would hide
/// the day one of them moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DueDiligenceSecondLife {
    /// An original battery, or one whose history is not established.
    ///
    /// The default for an unknown history, deliberately: the paragraph is an
    /// exemption, and wrongly excusing an operator is the worse error.
    #[default]
    None,
    /// Prepared for re-use, prepared for repurposing, repurposed or
    /// remanufactured, **and** already placed on the market or put into service
    /// before undergoing that operation. The caller asserts both halves by
    /// choosing this variant.
    PlacedBeforeOperations,
}
