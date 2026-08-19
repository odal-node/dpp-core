//! Ruleset identity and validity period types used across all `dpp-calc` calculators.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Opaque machine-readable identifier for a regulatory ruleset.
///
/// Examples: `"repairability-heuristic-v1"`, `"battery-cfb"`. Always a
/// compile-time literal in every concrete `Ruleset` impl in this crate — never
/// constructed from external/deserialized input — hence `&'static str` rather
/// than `String`: every ruleset's `id()`/`version()` becomes a plain `static`
/// instead of `OnceLock`-wrapped lazy-init ceremony.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct RulesetId(pub &'static str);

/// Semver-shaped version for a ruleset, tracking parameter changes across
/// delegated-act amendments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RulesetVersion(pub &'static str);

/// Whether, and from when, a ruleset version is legally in force.
///
/// The two variants are not "now" and "later" — they are "we know the date" and
/// "the date is not knowable yet". EU staged obligations are routinely written
/// as *"from &lt;date&gt; **or** N months after the entry into force of
/// &lt;act&gt;, **whichever is the latest**"* — see Regulation (EU) 2023/1542
/// Art. 7(1)–(3), Art. 8(1) and Art. 10(2)–(3). Until that act enters into
/// force the application date is genuinely unknown, and asserting one anyway
/// (a far-future sentinel, say) states a fact the regulation does not.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all_fields = "camelCase")]
pub enum Effectivity {
    /// In force from a known calendar date, until an optional end date.
    InForce {
        /// First day this ruleset version applies (inclusive).
        from: NaiveDate,
        /// Last day it applies (inclusive), or `None` if open-ended.
        until: Option<NaiveDate>,
    },
    /// The instrument that will date this ruleset has not entered into force,
    /// so no application date can be stated.
    Pending {
        /// The empowerment being waited on, e.g.
        /// `"EU 2023/1542 Art. 10(5), first subparagraph"`.
        empowerment: &'static str,
        /// The Commission's own deadline to adopt, where the regulation sets
        /// one. A deadline is **not** an application date — it can pass unmet,
        /// and several already have.
        adoption_deadline: Option<NaiveDate>,
    },
}

impl Effectivity {
    /// In force from `from`, open-ended.
    #[must_use]
    pub const fn open(from: NaiveDate) -> Self {
        Self::InForce { from, until: None }
    }

    /// In force for a closed period.
    #[must_use]
    pub const fn closed(from: NaiveDate, until: NaiveDate) -> Self {
        Self::InForce {
            from,
            until: Some(until),
        }
    }

    /// Awaiting an instrument that has not entered into force.
    #[must_use]
    pub const fn pending(empowerment: &'static str, adoption_deadline: Option<NaiveDate>) -> Self {
        Self::Pending {
            empowerment,
            adoption_deadline,
        }
    }

    /// Whether this ruleset governs `date`.
    ///
    /// A [`Pending`](Self::Pending) ruleset governs no date at all — not even a
    /// far-future one.
    #[must_use]
    pub fn is_active_on(&self, date: NaiveDate) -> bool {
        match self {
            Self::InForce { from, until } => date >= *from && until.is_none_or(|u| date <= u),
            Self::Pending { .. } => false,
        }
    }

    /// Error unless this ruleset governs `date`.
    ///
    /// Distinguishes three outcomes that must never be conflated: a ruleset that
    /// is **not yet effective** on a known date, one that has **expired**, and
    /// one whose date is **undetermined** because the instrument dating it has
    /// not entered into force.
    pub fn ensure_active_on(
        &self,
        id: &RulesetId,
        date: NaiveDate,
    ) -> Result<(), crate::error::CalcError> {
        match self {
            Self::Pending { empowerment, .. } => {
                Err(crate::error::CalcError::RulesetUndetermined {
                    id: id.0.to_owned(),
                    empowerment: (*empowerment).to_owned(),
                })
            }
            Self::InForce { from, until } => {
                if date < *from {
                    return Err(crate::error::CalcError::RulesetNotYetEffective {
                        id: id.0.to_owned(),
                        from: from.to_string(),
                    });
                }
                if let Some(until) = until
                    && date > *until
                {
                    return Err(crate::error::CalcError::RulesetExpired {
                        id: id.0.to_owned(),
                        until: until.to_string(),
                    });
                }
                Ok(())
            }
        }
    }
}

/// Structured legal citation for a regulatory ruleset.
///
/// Embedded in every [`Ruleset`] implementation so the authoritative source can
/// be located programmatically — without reading source comments or external docs.
/// This is the primary audit anchor for notified bodies.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegulatoryBasis {
    /// EU regulation or directive number (e.g. `"EU 2023/1669"`).
    pub regulation: &'static str,
    /// Relevant article and/or annex (e.g. `"Annex II, Annex III"`).
    pub article: &'static str,
    /// Harmonised standard used for the methodology (e.g. `"EN 45554:2021"`).
    pub standard: Option<&'static str>,
    /// JRC or other technical study underpinning the parameters (e.g. `"JRC128649"`).
    pub technical_study: Option<&'static str>,
    /// EUR-Lex or Official Journal URL for the authoritative source text.
    pub source_url: Option<&'static str>,
    /// Base ID of the successor ruleset version, if this one has been superseded.
    /// Set when the ruleset's `Effectivity::InForce.until` is populated.
    pub superseded_by: Option<&'static str>,
}

/// Common interface for all regulatory rulesets embedded in this crate.
///
/// There is deliberately no today-relative convenience method. A ruleset is
/// selected against the date the governing law attached to the product — see
/// [`AssessmentClock`](crate::clock::AssessmentClock) — and offering a
/// `…_today()` helper alongside would make the wrong answer the shorter one to
/// write.
pub trait Ruleset {
    fn id(&self) -> &RulesetId;
    fn version(&self) -> &RulesetVersion;
    fn effectivity(&self) -> &Effectivity;
    /// Structured citation for the EU regulation that mandates this calculation.
    fn regulatory_basis(&self) -> &RegulatoryBasis;
}
