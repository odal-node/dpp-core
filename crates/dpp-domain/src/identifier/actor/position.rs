//! [`ActorPosition`] — where in a passport an identifier sits.

/// Whether an identifier names an economic operator or a facility.
///
/// It is not part of the identifier's own shape, and a legacy `gln` string does
/// not say which it is, so the caller says. It decides one thing: which GS1
/// Application Identifier a GLN is qualified by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ActorPosition {
    /// An economic operator: Annex III points (g), (h) and (k).
    Operator,
    /// A facility: Annex III point (i).
    Facility,
}
