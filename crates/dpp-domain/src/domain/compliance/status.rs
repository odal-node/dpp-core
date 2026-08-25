//! [`ComplianceStatus`] — the overall determination, and the gate that keeps it
//! from being asserted without a legal basis.

use serde::{Deserialize, Serialize};

/// Overall compliance determination for a passport.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum ComplianceStatus {
    /// Manufacturer-supplied values stored verbatim — no calculation performed.
    PassthroughNoValidation,
    /// Calculated and compliant with applicable EU regulation.
    Compliant,
    /// Calculated; one or more fields fall below regulatory thresholds.
    NonCompliant,
    /// The product group's DPP obligation is not yet in force (provisional), so no
    /// binding determination is legally applicable — only structural validation
    /// was performed. See [`gate_determination`].
    NotAssessed,
    /// ProductGroup not yet implemented by this registry.
    NotImplemented,
}

impl ComplianceStatus {
    /// Every determination this build models, for exhaustive iteration.
    ///
    /// `ComplianceStatus` is `#[non_exhaustive]`, so a consumer outside this
    /// crate cannot enumerate it, and one publishing an API description has to.
    /// See [`crate::domain::seal::SealFormat::ALL`] for the same contract: a
    /// status added later is deliberately not covered until it is added here.
    pub const ALL: &'static [Self] = &[
        Self::PassthroughNoValidation,
        Self::Compliant,
        Self::NonCompliant,
        Self::NotAssessed,
        Self::NotImplemented,
    ];
}

/// Enforce regulatory status on a raw determination.
///
/// A product group no in-force act reaches may never surface a *binding*
/// `Compliant` / `NonCompliant` — there is no legal basis for the determination,
/// so it is downgraded to [`ComplianceStatus::NotAssessed`]. Groups an in-force
/// act does reach pass through unchanged, as do non-binding statuses.
///
/// Callers obtain `in_force` from
/// [`InstrumentCatalog::determinable_for`](crate::catalog::InstrumentCatalog::determinable_for),
/// which returns the (act, binding) pairs rather than a boolean. Pass the act
/// through to whatever records the result: a determination is always made under
/// a named instrument, and a caller that only learns "yes" cannot say which act
/// it is asserting against.
#[must_use]
pub fn gate_determination(in_force: bool, raw: ComplianceStatus) -> ComplianceStatus {
    if in_force {
        return raw;
    }
    match raw {
        ComplianceStatus::Compliant | ComplianceStatus::NonCompliant => {
            ComplianceStatus::NotAssessed
        }
        other => other,
    }
}
