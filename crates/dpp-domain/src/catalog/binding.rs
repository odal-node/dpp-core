//! [`InstrumentBinding`] — one act's reach into one product group.

use serde::{Deserialize, Serialize};

use super::granularity::Granularity;
use super::passport_obligation::PassportObligation;
use super::retention::RetentionBasis;
use super::status::RegulatoryStatus;

/// One instrument's reach into one product group, and the terms on which it
/// reaches it.
///
/// # Why the law lives here and not on the product group
///
/// A product group's `status`, `legalBasis`, passport date and retention period
/// used to be scalars on its own record, which asserts that exactly one act
/// governs it. ESPR Art. 5(7) says otherwise: a group-specific delegated act may
/// *supplement* a horizontal one, and no precedence rule exists anywhere in the
/// Regulation, so overlapping acts accumulate. Every one of those scalars is
/// therefore a property of an *(instrument, product group)* pair.
///
/// Concretely: a group is routinely in force under one instrument and
/// provisional under another, and a single boolean must then either assert
/// against an unadopted act or stay silent about an adopted one.
///
/// # Why bindings are held instrument-side
///
/// A binding is stored in the manifest of the **instrument**, listing the groups
/// it reaches — not in each group's manifest listing its instruments. Two
/// reasons, the second decisive:
///
/// 1. A horizontal act's coverage is a property of that act and moves as one
///    unit. Group-side, widening one act's scope means editing every group's
///    manifest.
/// 2. A horizontal act can reach a product group that has **no manifest of its
///    own** — the Commission's preparatory analysis says so explicitly, naming
///    light means of transport as a set covered by a horizontal requirement
///    while never being shortlisted as a product group. Group-side, that case
///    cannot be written down at all.
///
/// So [`product_group`](Self::product_group) is a free-form key and **is not
/// required to exist** in the product-group catalog. Resolving it is the
/// caller's business; the record of what an act reaches must not be lost merely
/// because we hold no schema for the thing reached.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstrumentBinding {
    /// Product-group key this instrument reaches. Not validated against the
    /// product-group catalog — see the type docs.
    pub product_group: String,
    /// Whether this instrument's obligations bind this group **now**, gating
    /// binding determinations made under this instrument.
    pub status: RegulatoryStatus,
    /// The specific provisions this binding rests on, e.g.
    /// `["Regulation (EU) 2023/1542 Art. 77(1)"]`. Article-level, because the
    /// binding is article-level: the act as a whole is
    /// [`Instrument::celex`](super::instrument::Instrument::celex).
    #[serde(default)]
    pub legal_basis: Vec<String>,
    /// Passport obligation scoped to this group, overriding the instrument's
    /// own where an act treats its groups differently. `None` inherits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub passport: Option<PassportObligation>,
    /// Retention period this instrument imposes on this group, in years.
    /// `None` inherits the instrument's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retention_years: Option<u32>,
    /// Whether [`Self::retention_years`] traces to an adopted text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retention_years_basis: Option<RetentionBasis>,
    /// The level this instrument fixes for this group. `None` inherits the
    /// instrument's, which is itself `None` where no act has fixed one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub granularity: Option<Granularity>,
    /// Scope, caveats, and the citation behind any date or figure above.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl InstrumentBinding {
    /// Whether a **binding compliance determination** may be made under this
    /// instrument for this product group.
    ///
    /// This is the same question [`RegulatoryStatus::allows_determination`] has
    /// always answered, moved to the altitude where it is answerable. It is
    /// **not** the question of whether a passport is required — see
    /// [`Self::requires_passport`]. Keeping them apart matters in both
    /// directions:
    ///
    /// - ESPR's unsold-goods duties bind today and are determinable, yet impose
    ///   no passport; and
    /// - an adjacent act's ecodesign duties bind today while its passport is
    ///   discharged through another system entirely.
    ///
    /// Reading one as the other in the second case is how a passport obligation
    /// that does not exist became assertable.
    #[must_use]
    pub fn allows_determination(&self) -> bool {
        self.status.allows_determination()
    }

    /// Whether this instrument requires a passport for this group, resolving the
    /// binding's override against the instrument's own obligation.
    #[must_use]
    pub fn requires_passport(&self, instrument_default: &PassportObligation) -> bool {
        self.passport
            .as_ref()
            .unwrap_or(instrument_default)
            .is_required()
    }
}
