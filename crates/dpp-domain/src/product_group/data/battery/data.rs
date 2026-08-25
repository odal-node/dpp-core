//! [`BatteryData`] — the Regulation (EU) 2023/1542 Annex XIII payload.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::super::common::CriticalRawMaterial;
use super::dynamic_performance::DynamicPerformance;
use super::expected_lifetime::ExpectedLifetime;
use super::hazardous_substance::{HazardSymbol, HazardousSubstance};
use super::material_composition::MaterialComposition;
use super::state_of_health::StateOfHealth;
use super::temperature_range::TemperatureRange;
use super::usage_history::UsageHistory;
use crate::identifier::Gtin;
use crate::product_group::{BatteryChemistry, BatteryStatus, BatteryType, CarbonFootprintClass};

/// Battery-specific fields required by the EU Battery Regulation 2023/1542.
///
/// All `Option` fields are optional under the regulation; non-`Option` fields
/// are mandatory for publishing a battery DPP. Fields added in v2.0.0 of the
/// schema are marked `Option` and `skip_serializing_if` to maintain backward
/// compatibility with v1.0.0 data.
///
/// # Superseded fields are kept, not deleted
///
/// The standing rule for this crate, and the reason `state_of_health_pct`,
/// `round_trip_efficiency_pct` and `internal_resistance_mohm` are all still
/// here:
///
/// > **A superseded field is marked legacy and retained. A field is deleted
/// > only when keeping it is itself the defect.**
///
/// Retention is the default because a stored record is entitled to keep its
/// value under the name it was written with. Re-homing it into a successor
/// asserts a correspondence the source never made — `internal_resistance_mohm`
/// cannot say whether it held the cell or the pack figure, so any lens that
/// picked one would invent the distinction the split exists to record. A legacy
/// field costs a doc comment and a `skip_serializing_if`; a wrong migration
/// costs the meaning of the data.
///
/// Deletion is reserved for the cases where the field's continued existence is
/// the harm rather than its obsolescence — `BatteryType::Other` silently
/// discarded unrecognised categories on round-trip, and the withdrawn IDTA and
/// ECLASS identifiers asserted another organisation's authority falsely. In
/// both, keeping the thing was the defect.
///
/// Legacy is marked by doc comment rather than `#[deprecated]`: the attribute
/// fires on every struct-literal construction, including every fixture, which
/// buys warning noise rather than safety in a struct nobody builds by accident.
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
    /// Capacity, in ampere-hours. **Annex VI Part A point 6** — "the capacity",
    /// mandatory for every battery category.
    ///
    /// Deliberately *not* Annex XIII point 1(g), "rated capacity (in Ah)",
    /// which the Commission's data-point guidance marks "not to be
    /// filled/displayed" for all three categories. The two are easy to
    /// conflate because this field is named for the second and required like
    /// the first. A third reading of the same quantity exists as a *measured*
    /// value in [`DynamicPerformance::rated_capacity_ah`]; that one is per
    /// battery, this one is per model.
    pub nominal_capacity_ah: f64,
    /// Expected lifetime in full charge–discharge cycles, as **declared for the
    /// model** — Annex XIII point 1(j).
    ///
    /// Optional because the duty is not universal: the obligation covers EV and
    /// LMT batteries but reaches industrial batteries only *"where lifetime can
    /// be expressed in cycles"*, and point 4(a) repeats the carve-out as
    /// *"except for non-cycle applications"*. A required field made an
    /// industrial battery with no meaningful cycle figure unrepresentable, so
    /// the constraint moves to a category-conditional rule rather than the
    /// schema. The *measured* counterpart is
    /// [`DynamicPerformance::expected_lifetime_cycles`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_lifetime_cycles: Option<u32>,
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

    /// Battery category per EU Battery Regulation 2023/1542 Art. 1(3).
    ///
    /// **Required since v2.5.0.** Annex VI Part A point 2, made public by
    /// Annex XIII point 1(a), lists "the battery category" as mandatory
    /// content of the publicly accessible tier — so a battery passport
    /// without one omits content the law requires. Earlier schema versions
    /// carry no equivalent field, which is why the v2.4.0 → v2.5.0 lens
    /// refuses rather than upgrading a record that predates the mandate.
    pub battery_type: BatteryType,

    /// Round trip energy efficiency when new, as a percentage — **Annex XIII
    /// point 1(n)**, the first of the two figures that point requires.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_round_trip_efficiency_pct: Option<f64>,

    /// Round trip energy efficiency *"at 50 % of cycle-life"*, as a percentage
    /// — **Annex XIII point 1(n)**, the second figure.
    ///
    /// The 50% is of **cycle-life**, not of state of charge. This field was
    /// previously named `round_trip_efficiency_pct` and documented as "at 50%
    /// state of charge", which names no data point in the regulation: 1(n) has
    /// no state-of-charge qualifier, and the only other round-trip efficiency
    /// in the annex is the measured one at point 4(a). Renamed so the condition
    /// travels with the field rather than in a comment that was wrong.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub round_trip_efficiency_at_half_cycle_life_pct: Option<f64>,

    /// **Legacy.** Round trip energy efficiency, unqualified — the single field
    /// that preceded the Annex XIII point 1(n) pair above.
    ///
    /// Retained so a record written against v2.5.0 keeps its value under its own
    /// name rather than being reinterpreted or refused. Its doc comment then
    /// read "at 50% state of charge", a condition 1(n) does not state, so the
    /// number's exact meaning is whatever the filer intended. New passports
    /// populate [`initial_round_trip_efficiency_pct`](Self::initial_round_trip_efficiency_pct)
    /// or
    /// [`round_trip_efficiency_at_half_cycle_life_pct`](Self::round_trip_efficiency_at_half_cycle_life_pct).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub round_trip_efficiency_pct: Option<f64>,

    /// **Legacy.** Internal resistance in milliohms, without saying whether it
    /// is the cell or the pack figure Annex XIII point 1(o) requires.
    ///
    /// That ambiguity is the reason the field was split, and the reason this one
    /// is kept rather than migrated: no rule can decide which measurement a
    /// stored value was, and inventing one would manufacture the distinction.
    /// Preserved verbatim; new passports populate
    /// [`internal_cell_resistance_mohm`](Self::internal_cell_resistance_mohm)
    /// and [`internal_pack_resistance_mohm`](Self::internal_pack_resistance_mohm).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_resistance_mohm: Option<f64>,

    /// Internal **cell** resistance in milliohms (mΩ) — **Annex XIII point
    /// 1(o)**, *"internal battery cell and pack resistance"*.
    ///
    /// Two measurements, not one. The single `internal_resistance_mohm` this
    /// replaces could not say which it held, and carried an invented "at 50%
    /// SoC" qualifier that 1(o) does not state. The *measured* per-battery
    /// resistance is a different data point and lives at
    /// [`DynamicPerformance::internal_resistance_mohm`], which the annex does
    /// state as a single value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_cell_resistance_mohm: Option<f64>,

    /// Internal **pack** resistance in milliohms (mΩ) — Annex XIII point 1(o),
    /// the other half of the same requirement.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_pack_resistance_mohm: Option<f64>,

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
    /// report them, so `ProductGroupData` should not carry their weight inline.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_of_health: Option<Box<StateOfHealth>>,

    // ── v2.6.0 — Annex VI Part A and Annex XIII point 1, public tier ────────
    /// Hazardous substances **other than** mercury, cadmium or lead — Annex VI
    /// Part A point 8. Those three are restriction thresholds in `dpp_rules`,
    /// not declared content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hazardous_substances: Option<Vec<HazardousSubstance>>,

    /// Extinguishing agent usable on this battery — Annex VI Part A point 9.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usable_extinguishing_agent: Option<String>,

    /// Share of renewable content, as a percentage — Annex XIII point 1(f).
    /// Distinct from recycled content, which point 1(e) covers separately.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub renewable_content_pct: Option<f64>,

    /// Minimal voltage in volts — Annex XIII point 1(h).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimal_voltage_v: Option<f64>,

    /// Maximum voltage in volts — Annex XIII point 1(h).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum_voltage_v: Option<f64>,

    /// The temperature range point 1(h) attaches to the voltage figures
    /// *"when relevant"*.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voltage_temperature_range: Option<TemperatureRange>,

    /// Original power capability in watts — Annex XIII point 1(i).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_power_capability_w: Option<f64>,

    /// Lower power limit in watts — Annex XIII point 1(i).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub power_limit_min_w: Option<f64>,

    /// Upper power limit in watts — Annex XIII point 1(i).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub power_limit_max_w: Option<f64>,

    /// The temperature range point 1(i) attaches to the power limits
    /// *"when relevant"*.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub power_temperature_range: Option<TemperatureRange>,

    /// The reference test the declared cycle lifetime was measured under —
    /// Annex XIII point 1(j), the other half of the requirement whose figure is
    /// [`BatteryData::expected_lifetime_cycles`]. A cycle count without its
    /// test is not comparable between manufacturers, which is why the annex
    /// asks for both.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_lifetime_reference_test: Option<String>,

    /// Capacity threshold for exhaustion, as a percentage of rated capacity —
    /// Annex XIII point 1(k). Mandatory for EV batteries only; the Commission's
    /// guidance marks it "not to be filled" for LMT and industrial.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capacity_threshold_for_exhaustion_pct: Option<f64>,

    /// Temperature range the battery can withstand **when not in use** — Annex
    /// XIII point 1(l).
    ///
    /// Not [`BatteryData::operating_temp_min_c`] and its pair, which describe
    /// the battery in service. The annex asks for both, and a battery tolerates
    /// more in storage than under load.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_in_use_temperature_range: Option<TemperatureRange>,

    /// The reference test the not-in-use range was measured under — the second
    /// half of Annex XIII point 1(l).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_in_use_temperature_reference_test: Option<String>,

    /// Period the commercial warranty for calendar life applies, in months —
    /// Annex XIII point 1(m), *"only if applicable (if commercial warranty
    /// envisaged)"*.
    ///
    /// A **duration**, not a date range: point 1 describes the model, and a
    /// warranty's start date is a property of an individual sale.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commercial_warranty_period_months: Option<u32>,

    /// C-rate of the relevant cycle-life test — Annex XIII point 1(p).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cycle_life_test_c_rate: Option<f64>,

    /// The markings applied under Art. 13(4) — Annex XIII point 1(q).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub marking_information: Option<String>,

    /// The Art. 13(5) chemical symbol, where one applies — Annex XIII point
    /// 1(q). The article names exactly two, so this is closed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hazard_symbol: Option<HazardSymbol>,

    /// Reference to the EU declaration of conformity under Art. 18 — Annex XIII
    /// point 1(r).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eu_declaration_of_conformity: Option<String>,

    /// Information on the prevention and management of waste batteries required
    /// by Art. 74(1) points (a) to (f) — Annex XIII point 1(s).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub waste_battery_information: Option<String>,

    // ── v2.6.0 — Annex XIII points 2 and 3, the non-public tiers ────────────
    // Point 2 reaches both non-public audiences; point 3 reaches authorities
    // only. Neither is projected to AAS: both are documents and contacts, and
    // that submodel is a technical-data snapshot.
    /// Part numbers for components — Annex XIII point 2(b).
    ///
    /// `restricted`: point 2 is available to notified bodies, market
    /// surveillance authorities **and** holders of a legitimate interest. The
    /// other half of 2(b) is [`BatteryData::spare_parts_contacts`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component_part_numbers: Option<Vec<String>>,

    /// Contact details of sources for replacement spares — Annex XIII point
    /// 2(b). `restricted`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spare_parts_contacts: Option<String>,

    /// Safety measures — Annex XIII point 2(d). `restricted`.
    ///
    /// Distinct from [`BatteryData::usable_extinguishing_agent`], which Annex VI
    /// Part A point 9 makes *public*: the same subject matter sits in two tiers
    /// because two annexes ask for it at two levels of detail.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_measures: Option<String>,

    /// Results of the test reports proving compliance with this Regulation and
    /// any act adopted under it — **Annex XIII point 3**.
    ///
    /// `conformity`: point 3 is the one tier reserved to authorities alone. A
    /// holder of a legitimate interest does not receive it, and neither does the
    /// public.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_report_results: Option<String>,

    // ── v2.6.0 — Annex XIII point 4, the rest of the individual-battery tier ─
    /// Measured performance and durability for **this** battery — Annex XIII
    /// point 4(a). See [`DynamicPerformance`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_performance: Option<Box<DynamicPerformance>>,

    /// Where this battery is in its life — Annex XIII point 4(c).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery_status: Option<BatteryStatus>,

    /// Recorded use history for **this** battery — Annex XIII point 4(d). See
    /// [`UsageHistory`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_history: Option<Box<UsageHistory>>,
}
