//! [`GlnAi`] — the GS1 Application Identifier that says what a GLN names.

use serde::{Deserialize, Serialize};

use super::position::ActorPosition;

/// The Application Identifier a scheme 7 GLN is qualified by, which is what tells a
/// party GLN from a location GLN.
///
/// The same thirteen digits can name a legal entity or a physical site, and the
/// number does not say which. EN 18219 clause 6.3 puts the distinction in the AI,
/// and Table C.11 of the standard gives the two used here: AI 417 for a party and
/// AI 414 for a physical location. GS1's own Syntax Dictionary names them `PARTY`
/// and `LOC No.`.
///
/// Serialises as the AI's digits.
///
/// 🚨 **AI 416, the GLN of a production or service location, is not here.**
/// The Syntax Dictionary defines it and Table C.11 does not list it, so whether a
/// facility may be named by it is not settled by anything read so far. Adding it
/// later moves nothing here, and a build that predates it reads a GLN under it as
/// an [`ActorIdentifier::Unrecognised`](super::ActorIdentifier::Unrecognised)
/// rather than refusing the passport that carries it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum GlnAi {
    /// AI 417: the GLN of a party, a legal entity or a functional entity.
    #[serde(rename = "417")]
    Party,
    /// AI 414: the GLN of a physical location.
    #[serde(rename = "414")]
    Location,
}

impl GlnAi {
    /// The AI's digits.
    #[must_use]
    pub const fn digits(self) -> &'static str {
        match self {
            Self::Party => "417",
            Self::Location => "414",
        }
    }

    /// The AI whose digits are `digits`, if it is one this build names.
    #[must_use]
    pub fn from_digits(digits: &str) -> Option<Self> {
        [Self::Party, Self::Location]
            .into_iter()
            .find(|ai| ai.digits() == digits)
    }

    /// The AI a GLN at `position` is qualified by: an operator is a party, and a
    /// facility is a physical location.
    ///
    /// An assumption about what a legacy `gln` string meant, since the string does
    /// not say. A facility that a party GLN names has to be stated as one.
    #[must_use]
    pub const fn for_position(position: ActorPosition) -> Self {
        match position {
            ActorPosition::Operator => Self::Party,
            ActorPosition::Facility => Self::Location,
        }
    }
}
