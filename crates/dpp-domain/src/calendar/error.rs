//! [`ScheduleBundleError`] — why a bundle's schedule was not read.

/// Why the obligation calendar did not read a bundle's schedule.
///
/// Either way the caller keeps the calendar it had. Nothing in a refused bundle
/// reaches the calendar, so a schedule is never applied in part.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ScheduleBundleError {
    /// The bundle was adopted without verification, as a node's compiled-in
    /// baseline is. A schedule is read only out of a bundle whose signature and
    /// content hash were both checked against the pinned publisher key.
    #[error(
        "bundle '{bundle_version}' was not verified, and an unverified bundle's schedule \
         is never read"
    )]
    Unverified {
        /// The refused bundle's version.
        bundle_version: String,
    },
    /// The bundle's schedule is not a list of well-formed entries with distinct
    /// ids.
    #[error("bundle '{bundle_version}' carries a malformed schedule: {reason}")]
    Malformed {
        /// The refused bundle's version.
        bundle_version: String,
        /// What is wrong with it.
        reason: String,
    },
}
