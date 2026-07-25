//! Typed inputs for the EU 2023/1669 repairability index.

use serde::{Deserialize, Serialize};

/// Lowest score the regulation assigns to any parameter.
pub const MIN_SCORE: u8 = 1;
/// Highest score the regulation assigns to any parameter.
pub const MAX_SCORE: u8 = 5;

/// Per-priority-part scores for one of the three part-level parameters
/// (disassembly depth, fasteners, tools).
///
/// Annex IV point 5 names ten priority parts. Each is scored 1–5 by that
/// parameter's own rubric.
///
/// Two rules from the regulation that the **caller** must apply before
/// populating this struct, because they need product knowledge this crate does
/// not have:
///
/// - if a priority part occurs more than once, supply the score of the
///   *lowest-scoring* instance;
/// - if a priority part is **not present** in the product, supply the highest
///   point level ([`MAX_SCORE`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    /// `None` for a non-foldable product. Presence selects the foldable weight
    /// set, so it must be consistent across all three part-level parameters.
    pub folding_mechanism: Option<u8>,
}

impl PriorityPartScores {
    /// Whether these scores describe a foldable product.
    #[must_use]
    pub fn is_foldable(&self) -> bool {
        self.folding_mechanism.is_some()
    }

    pub(crate) fn each(&self) -> [u8; 9] {
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
}

/// Complete input set for the repairability index of one smartphone or slate
/// tablet, per Regulation (EU) 2023/1669 Annex IV point 5.
///
/// The three part-level parameters carry per-part scores; the three
/// product-level parameters are single 1–5 scores.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairabilityIndexInputs {
    /// SDD — disassembly depth, scored per part from the number of steps.
    pub disassembly_depth: PriorityPartScores,
    /// SF — fasteners: reusable 5, resupplied 3, removable 1.
    pub fasteners: PriorityPartScores,
    /// ST — tools: none 5, basic 4, supplied with spare part 3, supplied with
    /// product 2, commercially available 1.
    pub tools: PriorityPartScores,
    /// SSP — spare-part availability, product level.
    pub spare_parts: u8,
    /// SSU — OS update duration: ≥7 years 5, 6 years 3, 5 years 1.
    pub software_updates: u8,
    /// SRI — repair-information availability, product level.
    pub repair_information: u8,
}
