//! [`ComplianceRegistry`](crate::ports::compliance::ComplianceRegistry) — dispatch to the strategy for a product group.

use chrono::NaiveDate;

use crate::domain::compliance::{ComplianceError, ComplianceResult};
use crate::domain::product_group::ProductGroupData;

/// Registry that dispatches to the correct `ComplianceStrategy` by product group.
///
/// The open-source default is `PassthroughRegistry`.
/// A proprietary binary can wire `PremiumComplianceRegistry` instead.
///
/// No `dpp-domain` code changes are required to swap implementations —
/// simply wire a different `Arc<dyn ComplianceRegistry>` at startup.
pub trait ComplianceRegistry: Send + Sync {
    /// Run compliance calculation for the given product group and data, under the law
    /// in force on `law_in_force_on` — see [`ComplianceStrategy::compute`](crate::ports::compliance::ComplianceStrategy::compute),
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
