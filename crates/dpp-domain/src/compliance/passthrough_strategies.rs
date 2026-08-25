//! The Apache-2.0 [`ComplianceStrategy`] implementations.
//!
//! Each one lifts the metrics a manufacturer supplied into a
//! [`ComplianceResult`] **verbatim**. None of them calculates, scores, or
//! decides anything: computing a determination is the job of the Wasm product group
//! plugins on the open-source path, or of a proprietary tier's own strategies.
//! Every result here therefore carries
//! [`ComplianceStatus::PassthroughNoValidation`](crate::ports::compliance::ComplianceStatus::PassthroughNoValidation)
//! and no findings.
//!
//! # Why these exist rather than a single product group-agnostic passthrough
//!
//! [`ComplianceRegistry`](crate::ports::compliance::ComplianceRegistry) is the
//! seam a proprietary binary swaps out wholesale;
//! [`ComplianceStrategy`] is the
//! seam it swaps out **one product group at a time**, which is the useful granularity:
//! a tier that computes a real battery determination still wants the passthrough
//! behaviour for the eleven product groups it does not model.
//!
//! That seam previously had no implementation anywhere — the trait was
//! published, documented as having two, and dispatched to by nothing. These are
//! those two, and [`PassthroughRegistry`](super::PassthroughRegistry) now
//! routes through them, so the extension point is exercised by the default
//! build rather than asserted by a doc comment.
//!
//! # Which metric goes where
//!
//! [`ComplianceResult`]'s three metric fields are product group-agnostic, and the
//! product group data types are not, so each strategy states its own mapping rather
//! than leaving it to be inferred:
//!
//! | Result field | Battery | Textile |
//! |---|---|---|
//! | `co2e_score` | `co2ePerUnitKg` | `carbonFootprintKgCo2e` |
//! | `recycled_content_pct` | — see below | `recycledContentPct` |
//! | `repairability_index` | not modelled | `repairScore` |

use chrono::NaiveDate;

use crate::domain::product_group::ProductGroupData;
use crate::ports::compliance::{
    ComplianceError, ComplianceErrorKind, ComplianceResult, ComplianceStrategy,
};

/// Battery passthrough — Regulation (EU) 2023/1542.
///
/// # Recycled content is deliberately absent
///
/// `ComplianceResult::recycled_content_pct` is one number.
/// Art. 8(2) and 8(3) set **per-metal** minima — cobalt, lead, lithium and
/// nickel — and `BatteryData` carries them as four separate fields because that
/// is what the regulation requires be documented. There is no defensible way to
/// collapse four into one: averaging them invents a figure the regulation never
/// asks for, and picking one silently drops three.
///
/// The measurement bases are not even the same. For cobalt, lithium and nickel
/// the share is measured "in active materials"; for lead it is the share
/// "present in the battery". A single percentage would conflate two
/// denominators as well as four metals.
///
/// So this strategy leaves the field `None` rather than filling it with
/// something arithmetically clean and regulatorily meaningless. The four values
/// travel where they belong — on `BatteryData`, in the passport, against their
/// own thresholds.
#[derive(Debug, Default, Clone, Copy)]
pub struct PassthroughBatteryStrategy;

impl ComplianceStrategy for PassthroughBatteryStrategy {
    fn product_group_key(&self) -> &str {
        "battery"
    }

    /// The governing-law date is ignored, and that is the correct behaviour for
    /// a passthrough: selecting a rule is what the date is for, and this
    /// selects none.
    fn compute(
        &self,
        data: &ProductGroupData,
        _law_in_force_on: Option<NaiveDate>,
    ) -> Result<ComplianceResult, ComplianceError> {
        let ProductGroupData::Battery(battery) = data else {
            return Err(wrong_product_group("battery", data));
        };
        Ok(ComplianceResult {
            // Manufacturer-declared, not computed. `co2e_per_unit_kg` is
            // non-optional on `BatteryData`, so there is always a value here.
            co2e_score: Some(battery.co2e_per_unit_kg),
            // See the type doc: four per-metal figures do not become one.
            recycled_content_pct: None,
            // Battery repairability has no EU index. `dpp-calc`'s EU 2023/1669
            // index covers smartphones and slate tablets, and its own module
            // doc says its heuristic is not comparable to it. Emitting either
            // here would put a number in a field a reader would take for a
            // regulatory score.
            repairability_index: None,
            ..ComplianceResult::passthrough()
        })
    }
}

/// Textile passthrough.
///
/// The product group is `provisional` in the catalog — no delegated act is in force —
/// so every field here is a manufacturer declaration against a data model that
/// is not yet ratified. `PassthroughNoValidation` is the only honest status, and
/// `gate_determination` would downgrade a binding one anyway.
#[derive(Debug, Default, Clone, Copy)]
pub struct PassthroughTextileStrategy;

impl ComplianceStrategy for PassthroughTextileStrategy {
    fn product_group_key(&self) -> &str {
        "textile"
    }

    /// Ignores the governing-law date, for the reason given on
    /// [`PassthroughBatteryStrategy::compute`].
    fn compute(
        &self,
        data: &ProductGroupData,
        _law_in_force_on: Option<NaiveDate>,
    ) -> Result<ComplianceResult, ComplianceError> {
        let ProductGroupData::Textile(textile) = data else {
            return Err(wrong_product_group("textile", data));
        };
        Ok(ComplianceResult {
            co2e_score: textile.carbon_footprint_kg_co2e,
            recycled_content_pct: textile.recycled_content_pct,
            // `repair_score` is the manufacturer's own 0–10 declaration, not the
            // EU 2023/1669 index — which does not apply to textiles at all.
            // Carried verbatim, which is what passthrough means.
            repairability_index: textile.repair_score,
            ..ComplianceResult::passthrough()
        })
    }
}

/// The error for a strategy handed data for a different product group.
///
/// A dispatch bug rather than bad user input, but it is reported as
/// [`ComplianceErrorKind::InvalidInput`] because from the strategy's position
/// that is exactly what it received, and because the alternative — panicking on
/// a mismatch — would make a routing mistake in a host take the process down.
fn wrong_product_group(expected: &str, got: &ProductGroupData) -> ComplianceError {
    ComplianceError {
        kind: ComplianceErrorKind::InvalidInput,
        message: format!(
            "{expected} strategy received {} data",
            got.product_group().catalog_key()
        ),
    }
}
