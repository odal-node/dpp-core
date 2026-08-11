//! Battery (EU Battery Regulation 2023/1542).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::gtin::Gtin;
use crate::domain::sector::enums::{
    BatteryChemistry, BatteryStatus, BatteryType, CarbonFootprintClass,
};

use super::shared::CriticalRawMaterial;

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
    /// report them, so `SectorData` should not carry their weight inline.
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

/// A temperature range in degrees Celsius.
///
/// Annex XIII asks for a range in three places — attached to the voltage
/// figures at point 1(h), to the power limits at 1(i), and standing alone at
/// 1(l) for the not-in-use case. One type rather than six loose bounds, so a
/// range cannot be half-declared.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct TemperatureRange {
    /// Lower bound in °C.
    pub min_c: f64,
    /// Upper bound in °C.
    pub max_c: f64,
}

/// A hazardous substance declared under Annex VI Part A point 8.
///
/// Deliberately **not** [`SvhcSubstance`](super::shared::SvhcSubstance), which
/// this crate already carries for textile, electronics and furniture. That type
/// is REACH-shaped — a concentration against the Art. 33 threshold, an ECHA
/// SCIP reference — and point 8 is a different instrument naming a different
/// set. Sharing the struct would assert the concepts are the same, which nobody
/// has established.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct HazardousSubstance {
    /// Substance name.
    pub name: String,
    /// CAS Registry Number, where the substance has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cas_number: Option<String>,
    /// Concentration in the battery as weight-%, where declared.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concentration_pct: Option<f64>,
}

/// The chemical symbol Art. 13(5) requires on a battery, where one applies.
///
/// Closed: Art. 13(5) names cadmium and lead and nothing else, and the
/// Commission's guidance records the data point as *"cadmium or lead symbol if
/// applicable"*. "No symbol required" is `Option::None`, which is why there is
/// no variant for it.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum HazardSymbol {
    Cadmium,
    Lead,
}

/// Measured performance and durability of one physical battery — Annex XIII
/// point 4(a).
///
/// # Why this is a block and not ten more fields on [`BatteryData`]
///
/// Point 4 describes **an individual battery**; points 1 to 3 describe a
/// **model**. The Commission's own data-point guidance makes the pairing
/// explicit rather than implicit — its entry for `ratedCapacityAh` here reads
/// *"same as data point number 11 (capacity), but now dynamic"* — so the same
/// quantity is deliberately carried twice, once as declared and once as
/// measured. Flattening these onto `BatteryData` would put the two readings
/// side by side distinguished only by name, and would let a filer put a
/// measured value in a declared field. Keeping the block separate makes the
/// distinction structural, and lets one `individual` disclosure entry cover
/// the whole set — the same shape [`StateOfHealth`] and [`ExpectedLifetime`]
/// already use.
///
/// # Optionality
///
/// Every field is `Option`. The guidance marks this set mandatory for EV and
/// LMT batteries but *"if applicable"* for industrial ones, and marks round
/// trip efficiency and its fade *"where applicable"* for all three. Which
/// fields a given battery owes is a category-conditional rule, not a schema
/// constraint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct DynamicPerformance {
    /// Rated capacity in ampere-hours, measured. The dynamic counterpart of
    /// [`BatteryData::nominal_capacity_ah`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rated_capacity_ah: Option<f64>,
    /// Capacity fade, as a percentage of the original rated capacity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capacity_fade_pct: Option<f64>,
    /// Power, in watts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub power_w: Option<f64>,
    /// Power fade, as a percentage of the original power.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub power_fade_pct: Option<f64>,
    /// Internal resistance, in milliohms.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_resistance_mohm: Option<f64>,
    /// Internal resistance increase, as a percentage of the original.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_resistance_increase_pct: Option<f64>,
    /// Energy round trip efficiency, as a percentage. *"Where applicable"* for
    /// every category, unlike the fields above.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub round_trip_efficiency_pct: Option<f64>,
    /// Energy round trip efficiency fade, as a percentage. *"Where
    /// applicable"*.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub round_trip_efficiency_fade_pct: Option<f64>,
    /// Expected lifetime under the reference conditions the battery was
    /// designed for, in cycles — *"except for non-cycle applications"*. The
    /// measured counterpart of [`BatteryData::expected_lifetime_cycles`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_lifetime_cycles: Option<u32>,
    /// The same expectation in **calendar years**, which the annex lists as a
    /// separate data point rather than a unit conversion: a battery can have a
    /// calendar-life expectation and no meaningful cycle count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_lifetime_years: Option<f64>,
}

/// Recorded use history of one physical battery — Annex XIII point 4(d).
///
/// Every item in 4(d) is *"if applicable"* for all three battery categories,
/// so nothing here is ever required by the schema.
///
/// **`negativeEvents` deliberately does not duplicate [`HarmfulEvents`].**
/// Annex VII Part B item 4 already requires harmful-event tracking as part of
/// the expected-lifetime parameter set, and this annex asks for the same
/// underlying facts under a different heading. Where a battery reports Part B
/// figures, [`ExpectedLifetime::harmful_events`] is the structured home and
/// this field carries what does not fit it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct UsageHistory {
    /// Number of charging and discharging cycles.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charge_discharge_cycles: Option<u32>,
    /// Negative events, such as accidents.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negative_events: Option<Vec<String>>,
    /// Periodically recorded operating environmental conditions, including
    /// temperature.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operating_conditions: Option<Vec<EnvironmentalReading>>,
    /// Periodically recorded state of charge, as a percentage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_of_charge: Option<Vec<StateOfChargeReading>>,
}

/// One periodic environmental observation — Annex XIII point 4(d).
///
/// The annex names temperature explicitly and leaves the rest of "operating
/// environmental conditions" open, so temperature is the one typed member and
/// anything further is recorded as a note rather than invented as a field.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct EnvironmentalReading {
    /// When the observation was taken.
    pub recorded_at: DateTime<Utc>,
    /// Temperature in °C.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature_c: Option<f64>,
    /// Any further condition the annex leaves unenumerated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// One periodic state-of-charge observation — Annex XIII point 4(d).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct StateOfChargeReading {
    /// When the observation was taken.
    pub recorded_at: DateTime<Utc>,
    /// State of charge as a percentage of usable capacity.
    pub state_of_charge_pct: f64,
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
