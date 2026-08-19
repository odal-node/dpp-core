//! Typed inputs for the Art. 8 recycled-content determination.

use serde::{Deserialize, Serialize};

use dpp_rules::batteries::recycled_content::RecycledContentInput;

/// The four declared recycled-content shares, in percent.
///
/// Each is `Option` because a share is a declaration the operator may not have
/// made, and an undeclared share is not a zero. A metal the battery's chemistry
/// does not contain is also `None` — scoping the declaration to the metals the
/// chemistry actually regulates is the caller's job, via
/// [`chemistry_regulated_metals`], and doing it before calling here is what
/// stops an LFP cell being told its cobalt share is short.
///
/// [`chemistry_regulated_metals`]: dpp_rules::batteries::recycled_content::chemistry_regulated_metals
///
/// # Two measurement bases, deliberately not unified
///
/// Art. 8(2) and 8(3) measure the cobalt, lithium and nickel shares "in active
/// materials", and the lead share as the share "present in the battery". The
/// four fields are therefore not four samples of one quantity and must never be
/// averaged into one — see the note on `ComplianceResult::recycled_content_pct`
/// in the domain crate for what that produces.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecycledContentInputs {
    /// Share of recycled cobalt in the active materials, percent.
    pub cobalt_pct: Option<f64>,
    /// Share of recycled lithium in the active materials, percent.
    pub lithium_pct: Option<f64>,
    /// Share of recycled nickel in the active materials, percent.
    pub nickel_pct: Option<f64>,
    /// Share of recycled lead present in the battery, percent.
    pub lead_pct: Option<f64>,
}

impl From<&RecycledContentInputs> for RecycledContentInput {
    fn from(i: &RecycledContentInputs) -> Self {
        Self {
            cobalt_pct: i.cobalt_pct,
            lithium_pct: i.lithium_pct,
            nickel_pct: i.nickel_pct,
            lead_pct: i.lead_pct,
        }
    }
}
