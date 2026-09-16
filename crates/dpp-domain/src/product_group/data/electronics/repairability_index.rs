//! The Annex IV point 5 inputs an operator declares, and the Art. 1 exclusion.
//!
//! ✅ COMPLIANCE-PIN: EU 2023/1669, Annex IV point 5 and Art. 1 (OJ L 214,
//! 31.8.2023, pp. 12 and 26). Read from the Official Journal text.
//!
//! # What this is, and what it deliberately is not
//!
//! **Declared inputs, not a score.** Annex IV point 5 computes the index as
//! `R = (SDD*0,25)+(SF*0,15)+(ST*0,15)+(SSP*0,15)+(SSU*0,15)+(SRI*0,15)` over
//! six parameters, three of them scored per priority part. This module carries
//! the six parameters as an operator states them. It does not compute `R`, and
//! nothing here grades anything.
//!
//! That split is the useful shape and it is the one the Regulation itself
//! describes: **Annex IX Table 10** sets a verification tolerance — *"The
//! determined value shall not be more than 4 % lower than the declared
//! value."* Declared by the supplier, re-determined by an authority. A passport
//! carrying the inputs, and a receipt recomputing the index from them, is the
//! same structure.
//!
//! # Why the shape is restated here rather than shared with the calculator
//!
//! The calculator's input type and this one describe the same six parameters and
//! are answerable to different things. This is **passport content** — a set of
//! values an operator declares, versioned with the electronics schema, and bound
//! by the signature over the record. The calculator's is a function signature.
//! Neither crate depends on the other by design, and that boundary is what keeps
//! a change to a calculation from silently changing what a passport means.
//!
//! The cost is real: the two can drift, and only a consumer wiring them together
//! would notice. Worth a shared home if a third consumer appears; not worth
//! inverting the crate graph for a second.
//!
//! # No obligation attaches to any of this
//!
//! **The word "passport" does not appear in Reg. (EU) 2023/1669.** The index
//! belongs on the energy label and in the product information sheet. Carrying it
//! here is a product decision, not a duty being discharged, and this note exists
//! so nobody later cites the Regulation for an obligation it does not create.

use super::priority_part_scores::{MAX_SCORE, MIN_SCORE, PriorityPartScores};
use serde::{Deserialize, Serialize};

/// The complete Annex IV point 5 input set for one smartphone or slate tablet.
///
/// Three part-level parameters carrying per-part scores, and three product-level
/// parameters that are single 1–5 scores. Every rubric below is Annex IV's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairabilityIndexDeclaration {
    /// SDD — disassembly depth, scored per part from the number of steps:
    /// ≤2 steps scores 5, >15 steps scores 1.
    ///
    /// Annex IV's counting rules apply, including the five-step penalty where
    /// reassembly needs a remote serial-number authorisation.
    pub disassembly_depth: PriorityPartScores,
    /// SF — fasteners: reusable 5, resupplied 3, removable 1.
    pub fasteners: PriorityPartScores,
    /// ST — tools: none 5, basic 4, supplied with the spare part 3, supplied
    /// with the product 2, commercially available 1.
    pub tools: PriorityPartScores,
    /// SSP — spare-part availability, product level, 1–5.
    pub spare_parts: u8,
    /// SSU — operating-system update duration, product level: ≥7 years scores 5,
    /// 6 years scores 3, 5 years scores 1.
    pub software_updates: u8,
    /// SRI — repair-information availability, product level, 1–5.
    pub repair_information: u8,
}

impl RepairabilityIndexDeclaration {
    /// Whether the three part-level parameters agree on whether the product folds.
    ///
    /// Annex IV selects between two weight sets on the presence of the hinge
    /// assembly, so a declaration scoring FM under one parameter and omitting it
    /// under another does not describe any product the annex can grade. Checked
    /// rather than assumed, because the failure is silent: the index would be
    /// computed against whichever weight set the first parameter happened to
    /// suggest.
    #[must_use]
    pub const fn foldable_is_consistent(&self) -> bool {
        let a = self.disassembly_depth.is_foldable();
        let b = self.fasteners.is_foldable();
        let c = self.tools.is_foldable();
        a == b && b == c
    }

    /// Whether every score, part-level and product-level, is within 1–5.
    #[must_use]
    pub fn scores_are_in_range(&self) -> bool {
        let in_range = |s: u8| (MIN_SCORE..=MAX_SCORE).contains(&s);
        self.disassembly_depth.scores_are_in_range()
            && self.fasteners.scores_are_in_range()
            && self.tools.scores_are_in_range()
            && in_range(self.spare_parts)
            && in_range(self.software_updates)
            && in_range(self.repair_information)
    }
}
