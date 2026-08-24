//! Open-core compliance boundary — strategy and registry traits.
//!
//! This module defines the extension seam used by proprietary compliance tiers.
//!
//! The open-source (Apache-2.0) binary wires `PassthroughRegistry`, which stores
//! manufacturer-supplied values verbatim without computing any scores.
//!
//! A proprietary binary can wire its own `PremiumComplianceRegistry`
//! implementation in a separate Cargo workspace without forking this crate.
//!
//! The value objects these traits produce — [`ComplianceResult`] and friends —
//! are domain values, not ports, and live in [`crate::domain::compliance`].
//! They are re-exported here so existing paths keep resolving.

use chrono::NaiveDate;

use crate::domain::product_group::ProductGroupData;

pub use crate::domain::compliance::{
    ComplianceError, ComplianceErrorKind, ComplianceFinding, ComplianceResult, ComplianceStatus,
    gate_determination,
};

// ─── Traits ───────────────────────────────────────────────────────────────

/// Per-product group compliance calculation strategy.
///
/// The Apache-2.0 build ships
/// [`PassthroughBatteryStrategy`](crate::compliance::PassthroughBatteryStrategy)
/// and
/// [`PassthroughTextileStrategy`](crate::compliance::PassthroughTextileStrategy),
/// both registered in
/// [`PassthroughRegistry::new`](crate::compliance::PassthroughRegistry::new). A
/// proprietary tier registers its own for the product groups it models and leaves the
/// rest on passthrough.
///
/// This is the **per-product group** seam;
/// [`ComplianceRegistry`] is the whole-registry one. The distinction is the
/// useful granularity: a tier that computes a real battery determination still
/// wants passthrough for the product groups it does not model, and swapping the
/// registry to get one product group means reimplementing dispatch for all of them.
///
/// # Contract
///
/// An implementation receives the [`ProductGroupData`] for **its own** product group and
/// must return [`ComplianceErrorKind::InvalidInput`] rather than panicking if
/// handed another's — a routing mistake in a host should be reportable, not
/// fatal.
///
/// # The governing-law date
///
/// `compute` takes the date the product was placed on the EU market, because a
/// strategy that computes anything must first decide *which rule applies*, and
/// that is a function of this date and never of today's. A strategy given only
/// the payload would have to read a clock to answer, and would then answer
/// differently on a Tuesday in 2031 than it did the day before, for a product
/// that had not changed.
///
/// It is `Option` because the date is a declaration a passport may omit, and
/// `None` is a real answer with a real consequence: the governing rule is
/// undetermined. It is **not** an invitation to substitute the current date.
///
/// The passthrough strategies ignore it, correctly — they compute nothing, so
/// there is no rule for them to select.
pub trait ComplianceStrategy: Send + Sync {
    /// The catalog key of the product group this strategy handles.
    ///
    /// A key rather than the `ProductGroup` enum: product group identity is catalog data,
    /// and a strategy for a product group this build has no variant for is exactly the
    /// case the open product group axis exists to allow.
    fn product_group_key(&self) -> &str;

    /// Compute a `ComplianceResult` from raw product group data, under the law in
    /// force on `law_in_force_on`.
    ///
    /// The passthrough implementation returns manufacturer-supplied values verbatim.
    /// A premium implementation runs calculations against EU methodology databases.
    fn compute(
        &self,
        data: &ProductGroupData,
        law_in_force_on: Option<NaiveDate>,
    ) -> Result<ComplianceResult, ComplianceError>;
}

/// Registry that dispatches to the correct `ComplianceStrategy` by product group.
///
/// The open-source default is `PassthroughRegistry`.
/// A proprietary binary can wire `PremiumComplianceRegistry` instead.
///
/// No `dpp-domain` code changes are required to swap implementations —
/// simply wire a different `Arc<dyn ComplianceRegistry>` at startup.
pub trait ComplianceRegistry: Send + Sync {
    /// Run compliance calculation for the given product group and data, under the law
    /// in force on `law_in_force_on` — see [`ComplianceStrategy::compute`],
    /// whose contract for that date this passes through unchanged.
    ///
    /// Returns `ComplianceErrorKind::UnknownProductGroup` if no strategy is registered
    /// for the requested product group.
    fn compute(
        &self,
        product_group_key: &str,
        data: &ProductGroupData,
        law_in_force_on: Option<NaiveDate>,
    ) -> Result<ComplianceResult, ComplianceError>;
}
