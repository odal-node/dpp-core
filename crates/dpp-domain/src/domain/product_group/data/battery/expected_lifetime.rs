//! [`ExpectedLifetime`] — the declared lifetime figures under Annex XIII.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use super::harmful_events::HarmfulEvents;

/// Expected-lifetime parameters per Annex VII **Part B**.
///
/// Part B is narrower than Part A: it names only *"stationary battery energy
/// storage systems and LMT batteries"*. Electric-vehicle batteries report a
/// state of health under Part A but no expected-lifetime parameter set here —
/// see `dpp_rules::batteries::degradation::annex_vii_part_b_applies_to`.
///
/// **Not the same thing as [`BatteryData::expected_lifetime_cycles`].** That
/// field is the model-level design figure Annex XIII point 1(j) makes *public*
/// ("expected battery lifetime expressed in cycles, and reference test used").
/// These are *measured* values for one physical battery, which Annex XIII point
/// 4(d) restricts to persons with a legitimate interest — hence the `individual`
/// disclosure class.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExpectedLifetime {
    /// 1. Date of putting into service, *"where appropriate"* — the only
    ///    qualified item in Part B. The date of manufacture, the other half of
    ///    item 1, is [`BatteryData::manufacturing_date`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub put_into_service_date: Option<NaiveDate>,
    /// 2. The energy throughput, in kWh.
    pub energy_throughput_kwh: f64,
    /// 3. The capacity throughput, in ampere-hours.
    pub capacity_throughput_ah: f64,
    /// 4. Tracking of harmful events.
    pub harmful_events: HarmfulEvents,
    /// 5. The number of full equivalent charge-discharge cycles.
    pub full_equivalent_cycles: f64,
}
