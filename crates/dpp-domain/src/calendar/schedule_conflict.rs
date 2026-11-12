//! [`ScheduleConflict`] — a bundle's date that this build's sourced date
//! overruled.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use super::BundleOrigin;

/// A date a signed bundle offered for an obligation whose date this build
/// already holds from an adopted text, where the two disagree.
///
/// The build's date stays on the calendar. This build's rules select their
/// answers by it, so announcing the bundle's date instead would tell an operator
/// one date while the publish gate applies another. A conflict is how the
/// disagreement stays visible: it usually means the law moved and this build has
/// not, which a new build fixes and a bundle cannot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleConflict {
    /// The obligation's id.
    pub id: String,
    /// The sourced date this build holds, and keeps.
    pub build_date: NaiveDate,
    /// The date the bundle offered.
    pub bundle_date: NaiveDate,
    /// The bundle that offered it.
    pub bundle: BundleOrigin,
}
