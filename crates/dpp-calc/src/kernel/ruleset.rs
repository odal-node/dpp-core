//! Ruleset identity and validity period types used across all `dpp-calc` calculators.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Opaque machine-readable identifier for a regulatory ruleset.
///
/// Examples: `"repairability-heuristic-v1"`, `"battery-cfb"`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RulesetId(pub String);

/// Semver-shaped version for a ruleset, tracking parameter changes across
/// delegated-act amendments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RulesetVersion(pub String);

/// The calendar range within which a ruleset version is legally valid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectiveDateBound {
    /// First day this ruleset version applies (inclusive).
    pub from: NaiveDate,
    /// Last day this ruleset version applies (inclusive), or `None` if open-ended.
    pub until: Option<NaiveDate>,
}

impl EffectiveDateBound {
    pub fn open(from: NaiveDate) -> Self {
        Self { from, until: None }
    }

    pub fn is_active_on(&self, date: NaiveDate) -> bool {
        date >= self.from && self.until.is_none_or(|u| date <= u)
    }

    /// Error if `date` falls outside the effective period, distinguishing a
    /// ruleset that is **not yet effective** (before `from`) from one that has
    /// **expired** (after `until`) — so a pending `2100` stub is never reported
    /// as "expired".
    pub fn ensure_active_on(
        &self,
        id: &RulesetId,
        date: NaiveDate,
    ) -> Result<(), crate::error::CalcError> {
        if date < self.from {
            return Err(crate::error::CalcError::RulesetNotYetEffective {
                id: id.0.clone(),
                from: self.from.to_string(),
            });
        }
        if let Some(until) = self.until
            && date > until
        {
            return Err(crate::error::CalcError::RulesetExpired {
                id: id.0.clone(),
                until: until.to_string(),
            });
        }
        Ok(())
    }
}

/// Structured legal citation for a regulatory ruleset.
///
/// Embedded in every [`Ruleset`] implementation so the authoritative source can
/// be located programmatically — without reading source comments or external docs.
/// This is the primary audit anchor for notified bodies.
#[derive(Debug, Clone, Serialize)]
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
    /// Set when `effective_dates.until` is populated.
    pub superseded_by: Option<&'static str>,
}

/// Common interface for all regulatory rulesets embedded in this crate.
pub trait Ruleset {
    fn id(&self) -> &RulesetId;
    fn version(&self) -> &RulesetVersion;
    fn effective_dates(&self) -> &EffectiveDateBound;
    /// Structured citation for the EU regulation that mandates this calculation.
    fn regulatory_basis(&self) -> &RegulatoryBasis;
}
