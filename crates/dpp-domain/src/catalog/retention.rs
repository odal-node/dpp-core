//! [`RetentionBasis`] — whether a sector's `retentionYears` traces to an
//! adopted legal text, or is carried as an assumption pending one.

use serde::{Deserialize, Serialize};

/// Whether [`crate::catalog::SectorDescriptor::retention_years`] is sourced
/// from an adopted legal text, or an assumption carried until one exists.
///
/// `retentionYears` is shipped data describing an operator's legal
/// obligation. A plausible number with no traceable basis is the same defect
/// class the claim-provenance work exists to prevent, so this marker exists
/// to make the distinction visible on the manifest itself rather than only in
/// a review note that can drift out of sync with the value it was about.
///
/// Deliberately binary rather than three-valued: a figure sourced for a
/// *different* obligation than passport availability (e.g. a documentation
/// retention period, not a passport one) is [`Self::Assumed`] here, not a
/// third state — it is not evidence for *this* claim, whatever else it
/// evidences. The manifest `notes` field carries that nuance where it applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RetentionBasis {
    /// An adopted legal text states this figure for passport availability.
    /// See the sector's `notes` for the citation.
    Sourced,
    /// No adopted legal text fixes this figure for passport availability yet.
    /// `retentionYears` is carried as a placeholder until one exists and must
    /// not be read as a legal minimum.
    Assumed,
}
