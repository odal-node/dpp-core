//! [`StateOfHealth`] — the two Annex VII state-of-health lists, kept apart.

use serde::{Deserialize, Serialize};

/// State-of-health parameters per Annex VII Part A of Reg. (EU) 2023/1542.
///
/// A sum type, not a struct of optionals, because Annex VII Part A is two
/// disjoint lists: electric-vehicle batteries report **state of certified
/// energy and nothing else**, while stationary battery energy storage systems
/// and LMT batteries report a five-parameter list. A flat struct would make "an
/// EV battery with an ohmic resistance but no SOCE" representable, which the
/// annex does not permit.
///
/// The optionality below is Annex VII's own wording, not a modelling choice:
/// items 1 and 4 of the stationary/LMT list are unconditional, while items 2, 3
/// and 5 are each qualified *"where possible"*.
///
/// Art. 14(1) has required these to be held in the battery management system
/// since 18 August 2024. Annex XIII point 4(b) makes state of health accessible
/// **only to persons with a legitimate interest** — so this field carries the
/// `individual` disclosure class and is withheld even from authorities.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "parameterSet", rename_all = "camelCase")]
pub enum StateOfHealth {
    /// Electric-vehicle batteries — Annex VII Part A, first list.
    #[serde(rename_all = "camelCase")]
    ElectricVehicle {
        /// State of certified energy (SOCE), as a percentage of the energy
        /// certified at manufacture.
        soce_pct: f64,
    },
    /// Stationary battery energy storage systems and LMT batteries —
    /// Annex VII Part A, second list.
    #[serde(rename_all = "camelCase")]
    StationaryOrLmt {
        /// 1. The remaining capacity, as a percentage of rated capacity.
        remaining_capacity_pct: f64,
        /// 2. *Where possible*, the remaining power capability (percentage).
        #[serde(skip_serializing_if = "Option::is_none")]
        remaining_power_capability_pct: Option<f64>,
        /// 3. *Where possible*, the remaining round trip efficiency (percentage).
        #[serde(skip_serializing_if = "Option::is_none")]
        remaining_round_trip_efficiency_pct: Option<f64>,
        /// 4. The evolution of self-discharging rates, in percent per month.
        ///    Unconditional in the annex, unlike items 2, 3 and 5.
        self_discharge_rate_pct_per_month: f64,
        /// 5. *Where possible*, the ohmic resistance, in milliohms.
        #[serde(skip_serializing_if = "Option::is_none")]
        ohmic_resistance_mohm: Option<f64>,
    },
}
