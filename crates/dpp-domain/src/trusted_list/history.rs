//! [`TrustServiceHistory`] — what a trust service's status was at a past moment.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::status::TrustServiceStatus;

/// One status, and the moment it took effect.
///
/// TS 119 612 clause 5.5.5 ("Current status starting date and time") for the
/// present status, and clause 5.6 ("Service history instance") for each previous
/// one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustServiceStatusPeriod {
    /// The status that began at [`Self::starting_at`].
    pub status: TrustServiceStatus,
    /// When this status took effect.
    pub starting_at: DateTime<Utc>,
}

/// The status of one trust service over time.
///
/// # Why this is not a single current status
///
/// **The question the law asks is in the past tense.** Regulation (EU)
/// No 910/2014 Art. 40 applies Art. 32 *mutatis mutandis* to seals, and
/// Art. 32(1)(b) asks whether the qualified certificate *"was issued by a
/// qualified trust service provider and was valid **at the time of signing**"*.
/// Not now. At the time of sealing.
///
/// The difference is the whole point of this type, and it is the easy mistake to
/// make, because "is this provider on the trusted list?" is the question a person
/// naturally asks and it has an answer that looks right. Consider a passport
/// sealed in 2027 by a provider whose qualified status was withdrawn in 2029:
///
/// - asking *"is it granted?"* in 2030 answers **no**, and reports a sound seal
///   as unqualified;
/// - asking *"was it granted in 2027?"* answers **yes**, which is what
///   Art. 32(1)(b) requires.
///
/// The reverse error is worse. A provider granted in 2029 was not qualified in
/// 2027, and a present-tense check would certify a seal that never was.
///
/// Trusted lists are built for this. TS 119 612 clause 5.3.12 requires the
/// retention period for historical information to be `65535`, which the standard
/// defines as retained indefinitely, and clause 5.5.1 note adds that historical
/// information "shall be retained even if the service's present status would not
/// normally" require it. The history is there precisely so that a past question
/// can be answered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustServiceHistory {
    /// Every recorded status, in any order.
    ///
    /// Not required to be sorted: a trusted list's history instances arrive in
    /// no guaranteed order, and requiring the caller to sort first is a
    /// precondition that would eventually be forgotten. [`Self::status_at`] does
    /// the ordering itself.
    pub periods: Vec<TrustServiceStatusPeriod>,
}

impl TrustServiceHistory {
    /// Build a history from its periods.
    #[must_use]
    pub fn new(periods: Vec<TrustServiceStatusPeriod>) -> Self {
        Self { periods }
    }

    /// The status in force at `when`, if the history covers that moment.
    ///
    /// The latest period whose start is at or before `when`. `None` when every
    /// recorded status began *after* `when` — meaning the list says nothing
    /// about that moment, which is different from saying the service was not
    /// granted then.
    ///
    /// That distinction is why this returns an `Option` rather than defaulting
    /// to withdrawn. "The trusted list does not go back that far" and "the
    /// service was not qualified" are different findings, and only the first
    /// leaves room for another source to answer.
    #[must_use]
    pub fn status_at(&self, when: DateTime<Utc>) -> Option<&TrustServiceStatus> {
        self.periods
            .iter()
            .filter(|p| p.starting_at <= when)
            .max_by_key(|p| p.starting_at)
            .map(|p| &p.status)
    }

    /// Whether the service held qualified status at `when`.
    ///
    /// False when the history does not reach back to `when` — an unknown status
    /// is not a granted one. Use [`Self::status_at`] where the difference
    /// between "not granted" and "not known" matters, which for a compliance
    /// finding it usually does.
    #[must_use]
    pub fn was_granted_at(&self, when: DateTime<Utc>) -> bool {
        self.status_at(when)
            .is_some_and(TrustServiceStatus::is_granted)
    }

    /// The status now, by the caller's clock.
    ///
    /// A convenience for reporting a provider's present standing — in a trust
    /// report, or when asking whether to buy from them. **Not** the question
    /// Art. 32(1)(b) asks about an existing seal; use [`Self::was_granted_at`]
    /// with the sealing time for that.
    #[must_use]
    pub fn current_status(&self, now: DateTime<Utc>) -> Option<&TrustServiceStatus> {
        self.status_at(now)
    }
}
