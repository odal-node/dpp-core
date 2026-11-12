//! [`LegalEntityIdentifier`] — how the disclosing undertaking is identified.

use serde::{Deserialize, Serialize};

/// The identifier of the legal entity making the disclosure.
///
/// **Annex I note (b) of Commission Implementing Regulation (EU) 2026/2:** it
/// "shall be the European unique identifier (`EUID`) established by Directive
/// (EU) 2017/1132 … or, **where not available**, any other identifier from an
/// officially recognised scheme in the Member State concerned."
///
/// An enum rather than a string plus a type field, because the two arms are not
/// symmetric: EUID needs no scheme name and the alternative is meaningless
/// without one. Annex I prints exactly this as a checkbox — "Type of identifier:
/// EUID | Other, namely: ___" — where the blank exists only on the second arm.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
#[non_exhaustive]
pub enum LegalEntityIdentifier {
    /// The European unique identifier, per Directive (EU) 2017/1132.
    Euid {
        /// The EUID itself.
        value: String,
    },
    /// An identifier from an officially recognised Member State scheme, used
    /// only where no EUID is available.
    Other {
        /// The scheme the identifier belongs to — Annex I's "namely:" blank.
        /// Without it the value cannot be resolved by a reader.
        scheme: String,
        /// The identifier itself.
        value: String,
    },
}

impl LegalEntityIdentifier {
    /// The identifier value, whichever scheme it belongs to.
    #[must_use]
    pub fn value(&self) -> &str {
        match self {
            Self::Euid { value } | Self::Other { value, .. } => value,
        }
    }
}
