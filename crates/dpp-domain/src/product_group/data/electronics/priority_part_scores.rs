//! [`PriorityPartScores`] — the ten priority parts Annex IV point 5 names, and
//! the 1–5 scale every parameter is scored on.
//!
//! ✅ COMPLIANCE-PIN: EU 2023/1669, Annex IV point 5 (OJ L 214, 31.8.2023,
//! p. 26). Read from the Official Journal text.

use serde::{Deserialize, Serialize};

/// Lowest score Annex IV point 5 assigns to any parameter.
pub const MIN_SCORE: u8 = 1;
/// Highest score Annex IV point 5 assigns to any parameter.
pub const MAX_SCORE: u8 = 5;

/// Per-priority-part scores for one of the three part-level parameters.
///
/// Annex IV point 5 names ten priority parts by abbreviation. Each is scored
/// 1–5 by the rubric of the parameter being scored.
///
/// Two rules from the Regulation the **operator** applies before declaring,
/// because they need product knowledge no code here has:
///
/// - a priority part occurring more than once takes the score of its
///   *lowest-scoring* instance;
/// - a priority part **not present** in the product takes [`MAX_SCORE`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PriorityPartScores {
    /// BAT — battery.
    pub battery: u8,
    /// DA — display assembly.
    pub display_assembly: u8,
    /// BC — back cover or back cover assembly.
    pub back_cover: u8,
    /// FFC — front-facing camera assembly.
    pub front_camera: u8,
    /// RFC — rear-facing camera assembly.
    pub rear_camera: u8,
    /// EC — external charging port.
    pub charging_port: u8,
    /// BUT — mechanical button.
    pub mechanical_button: u8,
    /// MIC — main microphone(s).
    pub microphone: u8,
    /// SPK — speaker.
    pub speaker: u8,
    /// FM — hinge assembly or mechanical display folding mechanism.
    ///
    /// `None` for a non-foldable product. Its presence selects the foldable
    /// weight set in Annex IV, so it must be stated consistently across all
    /// three part-level parameters — [`RepairabilityIndexDeclaration::foldable_is_consistent`](super::RepairabilityIndexDeclaration::foldable_is_consistent)
    /// is what checks that.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folding_mechanism: Option<u8>,
}

impl PriorityPartScores {
    /// Whether these scores describe a foldable product.
    #[must_use]
    pub const fn is_foldable(&self) -> bool {
        self.folding_mechanism.is_some()
    }

    /// The nine always-present part scores, in Annex IV's own order.
    #[must_use]
    pub const fn each(&self) -> [u8; 9] {
        [
            self.battery,
            self.display_assembly,
            self.back_cover,
            self.front_camera,
            self.rear_camera,
            self.charging_port,
            self.mechanical_button,
            self.microphone,
            self.speaker,
        ]
    }

    /// Whether every score present is within Annex IV's 1–5 range.
    #[must_use]
    pub fn scores_are_in_range(&self) -> bool {
        let in_range = |s: u8| (MIN_SCORE..=MAX_SCORE).contains(&s);
        self.each().iter().copied().all(in_range) && self.folding_mechanism.is_none_or(in_range)
    }
}
