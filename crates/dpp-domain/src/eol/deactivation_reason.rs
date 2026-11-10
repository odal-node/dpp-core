//! [`DeactivationReason`] — why a passport reached end of life.

use serde::{Deserialize, Serialize};

use super::derogation::Derogation;

/// Why a passport reached end of life.
///
/// A destruction cites the [`Derogation`] it relies on where there is one to cite.
/// Whether it must is [`EolEvent::missing_derogation`](super::EolEvent::missing_derogation)'s
/// question, because only the destruction of an unsold Annex VII product by an
/// operator the ban reaches needs one (ESPR Art. 25(1)): the type cannot know that,
/// and requiring a derogation of every destruction made the record claim one that
/// does not exist.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
#[non_exhaustive]
pub enum DeactivationReason {
    /// Sent for material recovery (preferred circular outcome).
    Recycled,
    /// Destroyed. Lawful without a derogation unless ESPR Art. 25(1) bars it.
    Destroyed {
        /// The derogation from the Art. 25 ban that authorises the destruction, if
        /// one applies. Absent is a real answer: the ban reaches only some
        /// destructions.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        derogation: Option<Derogation>,
    },
    /// Exported out of the EU market.
    Exported,
    /// Product lost (theft, disaster) — recorded, not silently dropped.
    Lost,
}

impl DeactivationReason {
    /// Every `kind` discriminator this build models, for exhaustive iteration.
    ///
    /// Discriminator strings rather than `&[Self]` because `Destroyed` carries a
    /// [`Derogation`] and so has no value-free form — the tag is the part a
    /// consumer needs to enumerate.
    ///
    /// `DeactivationReason` is `#[non_exhaustive]`, so a consumer outside this
    /// crate cannot enumerate it, and one publishing an API description has to.
    /// See [`crate::seal::SealFormat::ALL`] for the same contract: a
    /// reason added later is deliberately not covered until it is added here.
    pub const KINDS: &'static [&'static str] = &["recycled", "destroyed", "exported", "lost"];
}
