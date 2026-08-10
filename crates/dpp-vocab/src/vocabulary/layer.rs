//! [`Layer`] — what a vocabulary is *for*, which decides how it may be used.

use serde::{Deserialize, Serialize};

/// The role a vocabulary plays relative to our own model.
///
/// The distinction is not decoration. Confusing the three is how a data model
/// ends up owned by somebody else's release cycle: you build on a foundation,
/// you map to a profile, and you are tested by a bridge. A vocabulary treated
/// one layer higher than it belongs imports its churn into our wire format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Layer {
    /// A general vocabulary that predates the Digital Product Passport and
    /// outlives any single act. Something to **build on**.
    Foundational,
    /// A passport-specific model published by a community or programme, none of
    /// them normative. Something to **map to**.
    CommunityProfile,
    /// A domain-specific vocabulary scoped to one industry, sitting alongside
    /// the foundational layer rather than above it.
    Sectoral,
    /// A target we validate *against* rather than model *in*. Something we are
    /// **tested by**.
    ConformanceBridge,
}

impl Layer {
    /// Whether a term from this layer may be adopted as a prefix in our own
    /// serialisations, given that its vocabulary is otherwise verified.
    ///
    /// A conformance bridge never may: being measured against a model is not
    /// the same as speaking its language, and adopting a bridge's identifiers
    /// converts an external check into an internal dependency.
    #[must_use]
    pub const fn may_be_adopted_as_prefix(self) -> bool {
        match self {
            Self::Foundational | Self::CommunityProfile | Self::Sectoral => true,
            Self::ConformanceBridge => false,
        }
    }
}
