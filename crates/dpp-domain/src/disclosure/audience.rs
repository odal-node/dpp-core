//! [`Audience`] — who is asking for passport data.

use serde::{Deserialize, Serialize};

use super::class::{DISCLOSURE_ORDER, Disclosure, disclosure_key};

/// Who is asking for passport data.
///
/// Regulation (EU) 2023/1542 Art. 77(2) names three audiences and assigns each
/// a set of Annex XIII data points:
///
/// | Audience | Annex XIII |
/// |---|---|
/// | (a) general public | 1 |
/// | (b) notified bodies, market surveillance authorities, the Commission | 2 and 3 |
/// | (c) persons with a legitimate interest | 2 and 4 |
///
/// **This is a lattice, not a ranking.** Point 3 (conformity test reports) is
/// authority-only; point 4 (individual-item use history) is
/// legitimate-interest-only. Neither audience contains the other, so no integer
/// ordering can express the assignment: any `>=` comparison necessarily either
/// hands authorities the individual-item data Art. 77(2)(b) withholds, or hides
/// point-2 data from someone entitled to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Audience {
    /// Anyone, with no credential. Art. 77(2)(a).
    Public,
    /// A repairer, remanufacturer, second-life operator or recycler holding a
    /// credential that proves the interest. Art. 77(2)(c).
    LegitimateInterest,
    /// Notified body, market surveillance authority, customs, or the
    /// Commission. Art. 77(2)(b).
    Authority,
}

impl Audience {
    /// The disclosure classes this audience may see, in Annex XIII order.
    #[must_use]
    pub fn disclosure_set(self) -> Vec<Disclosure> {
        DISCLOSURE_ORDER
            .iter()
            .copied()
            .filter(|d| self.may_see(*d))
            .collect()
    }

    /// The [`disclosure_key`] for this audience's classes — the name under which
    /// a view served to it is signed and audited.
    ///
    /// Two audiences with the same class set would share a key, and that is
    /// correct: the artefact describes the data it covers, not who asked.
    #[must_use]
    pub fn disclosure_key(self) -> String {
        disclosure_key(&self.disclosure_set())
    }

    /// Whether this audience may see a field of class `disclosure`.
    ///
    /// The whole Art. 77(2) assignment, in one table.
    #[must_use]
    pub const fn may_see(self, disclosure: Disclosure) -> bool {
        matches!(
            (self, disclosure),
            (Self::Public, Disclosure::Public)
                | (
                    Self::LegitimateInterest,
                    Disclosure::Public | Disclosure::Restricted | Disclosure::Individual
                )
                | (
                    Self::Authority,
                    Disclosure::Public | Disclosure::Restricted | Disclosure::Conformity
                )
        )
    }
}
