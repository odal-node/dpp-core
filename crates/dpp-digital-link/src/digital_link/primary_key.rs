//! [`PrimaryKey`] — the AI that opens a GS1 Digital Link path.
//!
//! GS1 marks sixteen AIs as `dlpkey` in its Barcode Syntax Dictionary: `00`
//! (SSCC), `01` (GTIN), `253` (GDTI), `255` (GCN), `401` (GINC), `402` (GSIN),
//! `414` and `417` (party GLNs), `415` (pay-to GLN), `8003` (GRAI), `8004`
//! (GIAI), `8006` (ITIP), `8010` (CPID), `8013` (GMN), and `8017`/`8018`
//! (GSRN). A conformant Digital Link may open on any of them, and this crate
//! reads all sixteen.
//!
//! # One is a typed identifier and fifteen are not, and the API says so
//!
//! [`Gtin`] parses and check-digit validates, because this workspace models the
//! GTIN and that check traces to GS1's published mod-10 algorithm.
//!
//! The other fifteen are carried as strings. Each is held to what GS1's
//! dictionary says about its AI: its length, the character set of each component,
//! and the modulo-10 check digit where the entry names `csum` (AI `8003` is
//! `N1,zero N13,csum [X..16]`, so the digit covers the thirteen digits after the
//! filler, not the whole value), and the alphanumeric check character pair where
//! it names `csumalpha` (AI `8013`, a Global Model Number), and the four leading
//! digits of a GS1 Company Prefix where it names `gcppos1` or `gcppos2`. What is
//! **not** checked is whether that prefix is one GS1 has allocated, which needs
//! GS1's allocation data, and the rest of GS1's deeper validation.
//!
//! That asymmetry is a property a caller has to know about, so it is spelled
//! into the API rather than left in this paragraph. There is no method that
//! hands back "the value" for any key: [`PrimaryKey::as_gtin`] returns the
//! typed GTIN and nothing else, and [`PrimaryKey::unvalidated_value`] is the only
//! way to reach one of the other fifteen. A caller cannot read one without
//! writing the word, which says the value is not one of the typed identifiers
//! this workspace models, and does not say it was taken on trust.

use dpp_domain::Gtin;

/// The primary key an uncompressed Digital Link path opens on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrimaryKey {
    /// AI `01`, parsed and check-digit validated.
    Gtin(Gtin),
    /// Any other GS1 Digital Link primary key.
    ///
    /// The value is checked against the dictionary and no further — see the
    /// module doc for what that does and does not cover, and why the rest is a
    /// refusal to guess rather than an omission. The field is named for that, so
    /// a caller destructuring this variant reads it too.
    Other {
        /// The AI, exactly as GS1 spells it (`"00"`, `"8003"`).
        ai: String,
        /// The value as it appeared in the path, percent-decoded. Held to the
        /// dictionary's length, characters and check digit; the GS1 Company
        /// Prefix and a GMN's check characters are **not** verified.
        unvalidated_value: String,
    },
}

impl PrimaryKey {
    /// The AI that identifies this key.
    #[must_use]
    pub fn ai(&self) -> &str {
        match self {
            Self::Gtin(_) => "01",
            Self::Other { ai, .. } => ai,
        }
    }

    /// The validated GTIN, when this key is one.
    ///
    /// The only accessor that returns a checked identifier. Everything else
    /// this enum can hold comes back through [`Self::unvalidated_value`].
    #[must_use]
    pub fn as_gtin(&self) -> Option<&Gtin> {
        match self {
            Self::Gtin(g) => Some(g),
            Self::Other { .. } => None,
        }
    }

    /// The raw value of a key that is not a typed identifier.
    ///
    /// `None` for a GTIN — not because a GTIN has no value, but because it has
    /// a typed one, and reaching it through a method named `unvalidated` would
    /// make the name a lie for the one case where it does not apply. Use
    /// [`Self::as_gtin`] there.
    #[must_use]
    pub fn unvalidated_value(&self) -> Option<&str> {
        match self {
            Self::Gtin(_) => None,
            Self::Other {
                unvalidated_value, ..
            } => Some(unvalidated_value),
        }
    }

    /// The value as it belongs in a Digital Link path.
    ///
    /// Crate-internal on purpose: it is the one place the validated and
    /// unvalidated cases are treated alike, which is correct for writing a URI
    /// and wrong for anything that consumes the identifier. Exposing it would
    /// reintroduce the "just give me the value" accessor this type exists to
    /// avoid.
    #[must_use]
    pub(crate) fn wire_value(&self) -> &str {
        match self {
            Self::Gtin(g) => g.as_str(),
            Self::Other {
                unvalidated_value, ..
            } => unvalidated_value,
        }
    }
}
