//! [`ComplianceStrategy`] — the per-product-group determination seam.

use chrono::NaiveDate;

use crate::domain::compliance::{ComplianceError, ComplianceResult};
use crate::domain::product_group::ProductGroupData;

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
/// [`ComplianceRegistry`](crate::ports::compliance::ComplianceRegistry) is the whole-registry one. The distinction is the
/// useful granularity: a tier that computes a real battery determination still
/// wants passthrough for the product groups it does not model, and swapping the
/// registry to get one product group means reimplementing dispatch for all of them.
///
/// # Contract
///
/// An implementation receives the [`ProductGroupData`] for **its own** product group and
/// must return [`ComplianceErrorKind::InvalidInput`](crate::domain::compliance::ComplianceErrorKind::InvalidInput) rather than panicking if
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
