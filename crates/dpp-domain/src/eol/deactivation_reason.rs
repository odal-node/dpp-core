//! [`DeactivationReason`] — why a passport reached end of life.

use serde::{Deserialize, Serialize};

use super::derogation_ref::DerogationRef;

/// Why a passport reached end of life. Destruction requires a [`DerogationRef`]
/// so a record can never claim destruction without citing a lawful basis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
#[non_exhaustive]
pub enum DeactivationReason {
    /// Sent for material recovery (preferred circular outcome).
    Recycled,
    /// Destroyed — only lawful with a recognised derogation from the ban.
    Destroyed {
        /// The derogation category authorising destruction.
        derogation: DerogationRef,
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
    /// [`DerogationRef`] and so has no value-free form — the tag is the part a
    /// consumer needs to enumerate.
    ///
    /// `DeactivationReason` is `#[non_exhaustive]`, so a consumer outside this
    /// crate cannot enumerate it, and one publishing an API description has to.
    /// See [`crate::seal::SealFormat::ALL`] for the same contract: a
    /// reason added later is deliberately not covered until it is added here.
    pub const KINDS: &'static [&'static str] = &["recycled", "destroyed", "exported", "lost"];
}
