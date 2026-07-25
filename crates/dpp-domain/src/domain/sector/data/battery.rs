//! Battery (EU Battery Regulation 2023/1542).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::gtin::Gtin;
use crate::domain::sector::enums::{BatteryChemistry, BatteryType, CarbonFootprintClass};

use super::shared::CriticalRawMaterial;

/// Battery-specific fields required by the EU Battery Regulation 2023/1542.
///
/// All `Option` fields are optional under the regulation; non-`Option` fields
/// are mandatory for publishing a battery DPP. Fields added in v2.0.0 of the
/// schema are marked `Option` and `skip_serializing_if` to maintain backward
/// compatibility with v1.0.0 data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BatteryData {
    // ── v1.0.0 mandatory fields ──────────────────────────────────────────
    /// 14-digit Global Trade Item Number identifying the battery model.
    pub gtin: Gtin,
    /// Battery electrochemical chemistry.
    pub battery_chemistry: BatteryChemistry,
    /// Nominal voltage in volts.
    pub nominal_voltage_v: f64,
    /// Nominal capacity in ampere-hours.
    pub nominal_capacity_ah: f64,
    /// Expected lifetime in full charge–discharge cycles.
    pub expected_lifetime_cycles: u32,
    /// Carbon footprint in kg CO₂e per battery unit (manufacturer-supplied or calculated).
    pub co2e_per_unit_kg: f64,

    // ── v1.0.0 optional fields ───────────────────────────────────────────
    /// Recycled cobalt content as a percentage of total cobalt (0.0–100.0).
    pub recycled_content_cobalt_pct: Option<f64>,
    /// Recycled lithium content as a percentage of total lithium (0.0–100.0).
    pub recycled_content_lithium_pct: Option<f64>,
    /// Recycled nickel content as a percentage of total nickel (0.0–100.0).
    pub recycled_content_nickel_pct: Option<f64>,
    /// Current state of health as a percentage of original rated capacity.
    ///
    /// **Narrower than Annex VII Part A** — that annex defines state of health
    /// as state of certified energy for EV batteries and a five-parameter list
    /// for stationary storage and LMT, neither of which a single percentage can
    /// represent. Retained for schema versions up to v2.1.0; new passports
    /// should populate [`state_of_health`](Self::state_of_health) instead.
    pub state_of_health_pct: Option<f64>,
    /// Rated energy in kilowatt-hours (distinct from capacity in Ah).
    pub rated_capacity_kwh: Option<f64>,

    // ── v2.0.0 — Annex XIII compliance fields (Battery Reg. 2023/1542) ──
    /// Carbon footprint performance class label per Battery Regulation
    /// Art. 7(2), verbatim from the delegated act that established the scale.
    ///
    /// Meaningless without the two provenance fields below: the same label
    /// denotes different thresholds under different revisions of the scale, and
    /// Art. 7(2) requires those thresholds to be reviewed every three years.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub carbon_footprint_class: Option<CarbonFootprintClass>,

    /// Identifier of the ruleset whose class boundaries produced
    /// `carbon_footprint_class`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub carbon_footprint_class_ruleset_id: Option<String>,

    /// Version of the ruleset identified by `carbon_footprint_class_ruleset_id`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub carbon_footprint_class_ruleset_version: Option<String>,

    /// URL to supply chain due diligence documentation (Art. 47–52).
    /// Must link to a publicly accessible policy describing the due
    /// diligence process for raw material sourcing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_diligence_url: Option<String>,

    /// Cathode active material composition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cathode_material: Option<Vec<MaterialComposition>>,

    /// Anode active material composition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anode_material: Option<Vec<MaterialComposition>>,

    /// Electrolyte composition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub electrolyte_material: Option<Vec<MaterialComposition>>,

    /// Critical raw materials present (Art. 5(2)) — list of CAS or EC numbers.
    /// The EU Critical Raw Materials Act (2024/1252) defines the canonical list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub critical_raw_materials: Option<Vec<CriticalRawMaterial>>,

    /// URL or text for disassembly / dismantling instructions (Annex XIII §6).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disassembly_instructions_url: Option<String>,

    /// State-of-health determination methodology identifier, e.g.
    /// `"IEC 62660-1:2018"` or `"proprietary:vendor-model-v3"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub soh_methodology: Option<String>,

    /// Minimum operating temperature in °C.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operating_temp_min_c: Option<f64>,

    /// Maximum operating temperature in °C.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operating_temp_max_c: Option<f64>,

    /// Rated energy in watt-hours (Wh). Required by Annex XIII separately
    /// from `rated_capacity_kwh`. For cells this is the Wh stamping value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rated_energy_wh: Option<f64>,

    /// Recycled lead content as a percentage (for lead-acid batteries).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recycled_content_lead_pct: Option<f64>,

    /// Weight of the battery in kilograms.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery_weight_kg: Option<f64>,

    /// Battery type category per EU Battery Regulation 2023/1542 Art. 2.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery_type: Option<BatteryType>,

    /// Round-trip energy efficiency at 50% state of charge (percentage).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub round_trip_efficiency_pct: Option<f64>,

    /// Internal resistance in milliohms (mΩ) at 50% SoC.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_resistance_mohm: Option<f64>,

    // ── v2.1.0 ───────────────────────────────────────────────────────────
    /// Date the battery was placed on the EU market or put into service.
    ///
    /// Staged obligations attach by this date, never by the date of
    /// assessment: Art. 8(2) recycled-content minimums bind batteries placed
    /// on the market from 18 Aug 2031, Art. 8(3) from 18 Aug 2036, and
    /// Art. 10(4) disapplies the performance duties to batteries placed on the
    /// market before those duties applied. Without it, no phase determination
    /// is possible and none should be assumed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placed_on_market_date: Option<NaiveDate>,

    // ── v2.0.0 — Annex XIII identity & origin fields (Battery Reg. 2023/1542) ─
    /// Date and time of manufacture (Annex XIII §2 — "date of manufacture").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manufacturing_date: Option<DateTime<Utc>>,

    /// Plant / location of manufacture (ISO 3166-1 alpha-2 country code or
    /// "ISO country:city" free-text per Annex XIII §2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manufacturing_place: Option<String>,

    /// Manufacturer's battery model identifier as it appears on the physical label
    /// or accompanying technical documentation (Annex XIII §1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery_model_id: Option<String>,

    /// Unique battery passport identifier issued at commissioning.
    /// Format: per the Commission's implementing act on the battery passport
    /// (expected ~2026); until then a UUID v4 is accepted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery_passport_number: Option<String>,

    // ── v2.4.0 — Annex VII Part B expected lifetime ─────────────────────
    /// Measured expected-lifetime parameters per Annex VII Part B.
    ///
    /// Stationary storage and LMT only — Part B does not reach EV batteries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_lifetime: Option<Box<ExpectedLifetime>>,

    // ── v2.3.0 — Art. 8(1) declaration provenance ───────────────────────
    /// Calendar year the recycled-content shares pertain to.
    ///
    /// Art. 8(1) requires the shares "for each battery model **per year and per
    /// manufacturing plant**", so a percentage without both anchors is not the
    /// Art. 8(1) declaration — it is an unattributed number. The plant is
    /// `manufacturing_place`; this is the year.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recycled_content_reporting_year: Option<u16>,

    // ── v2.2.0 — Annex VII Part A state of health ────────────────────────
    /// State-of-health parameters per Annex VII Part A, in the parameter set
    /// its battery category requires. Supersedes `state_of_health_pct`.
    ///
    /// Boxed: the two Annex VII measurement blocks are large relative to the
    /// rest of `BatteryData` and populated only for batteries that actually
    /// report them, so `SectorData` should not carry their weight inline.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_of_health: Option<Box<StateOfHealth>>,
}

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

/// Harmful events tracked under Annex VII Part B item 4.
///
/// The annex says *"the tracking of harmful events, **such as** the number of
/// deep discharge events, time spent in extreme temperatures, time spent
/// charging in extreme temperatures"*. "Such as" makes that list illustrative,
/// not closed — so every field here is optional, and an implementation tracking
/// a further event type is conforming, not extending.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct HarmfulEvents {
    /// Number of deep discharge events.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deep_discharge_events: Option<u32>,
    /// Cumulative hours spent outside the battery's rated temperature range.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hours_in_extreme_temperature: Option<f64>,
    /// Cumulative hours spent *charging* outside that range.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hours_charging_in_extreme_temperature: Option<f64>,
}

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

/// Material composition entry for cathode, anode, or electrolyte.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MaterialComposition {
    /// Chemical name or formula, e.g. `"LiFePO4"`, `"graphite"`, `"LiPF6"`.
    pub name: String,
    /// Weight percentage in the component (0.0–100.0).
    pub weight_pct: f64,
    /// CAS Registry Number if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cas_number: Option<String>,
}
