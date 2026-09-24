//! [`CarrierQualifier`] — what a passport's data carrier prints after its GTIN.

use std::borrow::Cow;

/// What a passport's GS1 data carrier prints after its GTIN, and so the level
/// the carrier claims to identify.
///
/// # Why the level decides it
///
/// A GS1 Digital Link keyed on a GTIN names a different thing depending on the
/// qualifier that follows. The GTIN alone names the trade item — the model. With
/// AI 10 it names one batch or lot of it. With AI 21 it is a serialised GTIN,
/// which GS1 defines as identifying **one individual** item: two otherwise
/// identical units carry distinct serialised GTINs. A carrier printed on every
/// unit a model-level passport covers therefore must not carry AI 21, or every
/// one of those units presents the same "individual" identity.
///
/// Art. 10(1)(f) of Regulation (EU) 2024/1781 draws the same three-way line —
/// the data *"shall refer to the product model, batch or item"* — and
/// [`Granularity`](crate::Granularity) records which of the three a passport is.
/// [`Passport::carrier_qualifier`](crate::Passport::carrier_qualifier) maps one
/// onto the other.
///
/// # One value, read twice
///
/// The carrier is built from this, and a printed label is resolved by comparing
/// against it, so the two cannot drift: a carrier that asserts one level and a
/// lookup that infers another is how a label came to resolve to nothing before.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CarrierQualifier<'a> {
    /// No qualifier: `/01/{gtin}`. The passport covers every unit of a model.
    Model,
    /// AI 10: `/01/{gtin}/10/{batch}`. The passport covers one production run.
    Batch(Cow<'a, str>),
    /// AI 21: `/01/{gtin}/21/{serial}`. The passport covers one unit, and the
    /// serial is its [carrier serial](crate::Passport::effective_carrier_serial).
    Serial(Cow<'a, str>),
}

impl CarrierQualifier<'_> {
    /// The GS1 element this qualifier prints after the GTIN, as
    /// `(application identifier, value)`, or `None` for [`Self::Model`], which
    /// prints nothing.
    ///
    /// Answered here rather than by a match in the crate that builds the link,
    /// because this enum is `#[non_exhaustive]`: a match from another crate
    /// needs a wildcard, and a wildcard would print a future level as one of
    /// these three without anyone deciding that it should.
    #[must_use]
    pub fn ai_element(&self) -> Option<(&'static str, &str)> {
        match self {
            Self::Model => None,
            Self::Batch(batch) => Some(("10", batch)),
            Self::Serial(serial) => Some(("21", serial)),
        }
    }
}
