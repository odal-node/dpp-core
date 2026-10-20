//! [`ActorIdentifier`] — an operator or facility identifier, typed by clause 6 scheme.

use dpp_rules::common::identifier::{is_duns, is_valid_lei};
use serde::{Deserialize, Serialize};

use super::super::gln::Gln;
use super::error::ActorIdentifierError;
use super::gln_ai::GlnAi;
use super::icd::{DUNS, Icd, LEI};
use super::position::ActorPosition;

/// An economic operator or facility identifier, in whichever EN 18219 clause 6
/// scheme it follows.
///
/// ✅ COMPLIANCE-PIN: EN 18219:2026 clause 6.1 — an operator or facility
/// identifier, where regulation requires one, meets the general requirements of
/// clause 4 and follows one of the schemes of clause 6. Two are typed here.
///
/// - **Scheme 6** (clause 6.2), [`Self::Icd`]: an identifier in a scheme registered
///   under ISO/IEC 6523, told apart from other schemes by its ICD. The standard
///   names the LEI (ICD 0199) and the D-U-N-S number (0060) among its examples,
///   and this type checks the shape of those two. Any other ICD is accepted with
///   a non-blank value and **no further check**, since the syntax of a scheme is
///   the scheme's own and this crate does not hold it.
/// - **Scheme 7** (clause 6.3), [`Self::Gs1`]: a GS1 Application Identifier. A
///   GLN is carried with the AI that says whether it names a party (417) or a
///   physical location (414), so the two are told apart by what the identifier
///   says and not by where it was found.
///
/// A GLN is *also* admissible under scheme 6, as ICD 0088, and the standard lists
/// both. This type chooses scheme 7 for it, because only the AI carries the
/// party/location distinction.
///
/// Not here, and not an oversight: scheme 8, a DID that a register has issued a
/// credential to (clause 6.4), and scheme 9, a DOI (clause 6.5). The enum is
/// `#[non_exhaustive]` so either can be added without moving what is here. A DID
/// that an operator names for itself, with no register credential behind it, is
/// not a clause 6 identifier, so a legacy `did` string is not read as one.
///
/// # What serialising and reading mean
///
/// ```json
/// { "scheme": "icd", "icd": "0199", "value": "529900T8BM49AURSDO55" }
/// { "scheme": "gs1", "ai": "417",   "gln":   "9506000134352" }
/// ```
///
/// 🚨 **Reading is not issuing.** The constructors check a value's shape and
/// refuse a bad one. Deserialization checks only that the document is the right
/// *shape*: four digits for an ICD, a valid GLN for a GLN, since each is
/// validated by its own type. It does **not** apply the LEI checksum or the
/// D-U-N-S digit count. An operator identifier is part of a passport that can
/// arrive from another operator, signed and unrewritable, and a reader that
/// refused a value it could not have refused when it was written would refuse
/// correct data permanently. What stops a bad value persisting is the write path:
/// it goes through [`Self::iso6523`], and [`Self::validate`] is there to ask
/// again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "scheme", rename_all = "camelCase")]
#[non_exhaustive]
pub enum ActorIdentifier {
    /// **Scheme 6** (clause 6.2): an identifier registered under ISO/IEC 6523.
    Icd {
        /// The International Code Designator of the scheme the value is in.
        icd: Icd,
        /// The identifier, in that scheme's own syntax.
        value: String,
    },
    /// **Scheme 7** (clause 6.3), the GS1 route: a GLN and the AI that says what
    /// it names.
    Gs1 {
        /// Whether the GLN names a party or a physical location.
        ai: GlnAi,
        /// The Global Location Number, check digit verified.
        gln: Gln,
    },
}

impl ActorIdentifier {
    /// A scheme 6 identifier: `value` in the ISO/IEC 6523 scheme named by `icd`.
    ///
    /// The value is checked for shape under the two schemes whose syntax this
    /// crate holds, an LEI under ICD 0199 and a D-U-N-S number under 0060.
    /// Under any other ICD it is checked only to be non-blank.
    ///
    /// # Errors
    ///
    /// [`ActorIdentifierError::BlankValue`] for an empty value, and
    /// [`ActorIdentifierError::InvalidValue`] when the value is not an LEI or a
    /// D-U-N-S number under the ICD that says it is.
    pub fn iso6523(icd: Icd, value: &str) -> Result<Self, ActorIdentifierError> {
        let identifier = Self::Icd {
            icd,
            value: value.to_owned(),
        };
        identifier.validate()?;
        Ok(identifier)
    }

    /// A scheme 7 identifier: a GLN qualified by the AI that says what it names.
    #[must_use]
    pub fn gs1(ai: GlnAi, gln: Gln) -> Self {
        Self::Gs1 { ai, gln }
    }

    /// Read a legacy `(scheme, value)` pair, as an operator or facility
    /// identifier is held today, and say which clause 6 scheme it is.
    ///
    /// This is the one place those strings are read. What it places:
    ///
    /// | legacy `scheme` | read as |
    /// |---|---|
    /// | `lei` | scheme 6, ICD 0199, the value checked as an LEI |
    /// | `duns` | scheme 6, ICD 0060, the value checked as nine digits |
    /// | `gln` | scheme 7, AI 417 at an operator and AI 414 at a facility, the value checked as a GLN |
    ///
    /// Everything else is [`ActorIdentifierError::Unmapped`], including `vat`,
    /// `eori`, `did`, `national` and any string not listed. That is not a verdict
    /// on the identifier. It means this build knows of no clause 6 scheme to read
    /// it as: an EU-wide VAT number has no ICD, and none was found for an EORI
    /// number, so reading them as something would be a guess presented as a
    /// mapping. A `did` is scheme 8 only with a register-issued credential, which
    /// is not modelled here.
    ///
    /// The scheme string is matched exactly, as the registry matches it.
    ///
    /// # Errors
    ///
    /// [`ActorIdentifierError::Unmapped`] for a scheme this build cannot place,
    /// and [`ActorIdentifierError::InvalidValue`], [`BlankValue`](ActorIdentifierError::BlankValue)
    /// or [`InvalidGln`](ActorIdentifierError::InvalidGln) when it can place the
    /// scheme and the value does not fit it.
    pub fn from_legacy(
        scheme: &str,
        value: &str,
        position: ActorPosition,
    ) -> Result<Self, ActorIdentifierError> {
        match scheme {
            "lei" => Self::iso6523(Icd::lei(), value),
            "duns" => Self::iso6523(Icd::duns(), value),
            "gln" => {
                let gln = Gln::parse(value).map_err(|error| ActorIdentifierError::InvalidGln {
                    value: value.to_owned(),
                    reason: error.to_string(),
                })?;
                Ok(Self::gs1(GlnAi::for_position(position), gln))
            }
            other => Err(ActorIdentifierError::Unmapped {
                scheme: other.to_owned(),
            }),
        }
    }

    /// Check the value again, as [`Self::iso6523`] does when it is built.
    ///
    /// For a value that arrived by deserialization, which checks only the shape
    /// of the document and not the value: see the type's note on reading.
    ///
    /// # Errors
    ///
    /// As [`Self::iso6523`]. A scheme 7 identifier holds a [`Gln`] that was
    /// validated when it was made, so it cannot fail here.
    pub fn validate(&self) -> Result<(), ActorIdentifierError> {
        let Self::Icd { icd, value } = self else {
            return Ok(());
        };
        if value.trim().is_empty() {
            return Err(ActorIdentifierError::BlankValue);
        }
        let fits = match icd.as_str() {
            LEI => is_valid_lei(value),
            DUNS => is_duns(value),
            // The syntax of any other scheme is the scheme's own.
            _ => true,
        };
        if fits {
            Ok(())
        } else {
            Err(ActorIdentifierError::InvalidValue {
                icd: icd.as_str().to_owned(),
                value: value.clone(),
            })
        }
    }

    /// The EN 18219 scheme number this identifier follows: 6 or 7.
    #[must_use]
    pub fn en_18219_scheme(&self) -> u8 {
        match self {
            Self::Icd { .. } => 6,
            Self::Gs1 { .. } => 7,
        }
    }

    /// The identifier's value, without its scheme.
    #[must_use]
    pub fn value(&self) -> &str {
        match self {
            Self::Icd { value, .. } => value,
            Self::Gs1 { gln, .. } => gln.as_str(),
        }
    }
}
