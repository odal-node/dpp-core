//! [`PrimaryKey`] — the AI that opens a GS1 Digital Link path.
//!
//! GS1 marks sixteen AIs as `dlpkey` in its Barcode Syntax Dictionary: `00`
//! (SSCC), `01` (GTIN), `253` (GDTI), `255` (GCN), `401` (GINC), `402` (GSIN),
//! `414` and `417` (party GLNs), `415` (pay-to GLN), `8003` (GRAI), `8004`
//! (GIAI), `8006` (ITIP), `8010` (CPID), `8013` (GMN), and `8017`/`8018`
//! (GSRN). A conformant Digital Link may open on any of them, and this crate
//! reads all sixteen.
//!
//! # One is validated and fifteen are not, and the API says so
//!
//! [`Gtin`] parses and check-digit validates, because this workspace models the
//! GTIN and that check traces to GS1's published mod-10 algorithm.
//!
//! The other fifteen are carried as strings, length-checked against the
//! dictionary and **not check-digit validated**. Most declare a `csum`, but
//! what the check digit covers varies per key: AI `8003` is
//! `N1,zero N13,csum [X..16]`, so the digit covers the middle thirteen digits
//! and not the value; AI `8013` is `csumalpha`, a different algorithm
//! altogether. Writing those from the shape of a dictionary line, without the
//! specification that defines each one, is the invented detail this project
//! refuses elsewhere — and a check that is wrong in the permissive direction is
//! worse than none, because it reports a validation that did not happen.
//!
//! That asymmetry is a property a caller has to know about, so it is spelled
//! into the API rather than left in this paragraph. There is no method that
//! hands back "the value" for any key: [`PrimaryKey::as_gtin`] returns the
//! validated GTIN and nothing else, and [`PrimaryKey::unvalidated_value`] is
//! the only way to reach one of the other fifteen. A caller cannot read one
//! without writing the word.

use dpp_domain::Gtin;

/// The primary key an uncompressed Digital Link path opens on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrimaryKey {
    /// AI `01`, parsed and check-digit validated.
    Gtin(Gtin),
    /// Any other GS1 Digital Link primary key.
    ///
    /// The value is length-checked against the dictionary and otherwise
    /// unverified — see the module doc for why that is a refusal to guess
    /// rather than an omission. The field is named for that, so a caller
    /// destructuring this variant reads it too.
    Other {
        /// The AI, exactly as GS1 spells it (`"00"`, `"8003"`).
        ai: String,
        /// The value as it appeared in the path, percent-decoded. **No check
        /// digit has been verified.**
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

    /// The raw value of a key that is **not** check-digit validated.
    ///
    /// `None` for a GTIN — not because a GTIN has no value, but because it has
    /// a validated one, and reaching it through a method named `unvalidated`
    /// would make the name a lie for the one case where it does not apply. Use
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
