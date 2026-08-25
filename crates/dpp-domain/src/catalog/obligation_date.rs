//! [`ObligationDate`] — when a passport obligation begins, and on whose authority.

use serde::{Deserialize, Serialize};

/// Whether a date was read from an adopted text or is carried as an assumption.
///
/// The same distinction [`RetentionBasis`](crate::catalog::RetentionBasis) draws
/// for retention figures, generalised — because the failure it prevents is the
/// same one, and it has already happened once here. A date inferred from an
/// *ecodesign* application date was shipped as a **passport** application date,
/// and nothing in the record said it was inferred. A plausible date with no
/// traceable source is indistinguishable from a sourced one unless the type
/// makes the difference visible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DateBasis {
    /// An adopted legal text states this date for the passport obligation. The
    /// citation belongs in the surrounding record's `notes`.
    Sourced,
    /// No adopted text fixes this date. Carried as a working assumption and must
    /// not be presented as a legal deadline.
    Assumed,
}

/// The date a passport obligation begins, with the provenance of that date.
///
/// A struct rather than two loose fields so a date cannot exist without its
/// basis: [`PassportObligation::Required`](crate::catalog::PassportObligation::Required) with no date at all is a legitimate
/// state — the act mandates a passport and has not yet fixed when — and it
/// leaves no orphaned basis behind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObligationDate {
    /// ISO-8601 date the passport obligation applies from.
    pub date: String,
    /// Whether [`Self::date`] traces to an adopted text.
    pub basis: DateBasis,
}
