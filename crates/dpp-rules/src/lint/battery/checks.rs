//! Plausibility checks on a battery's declared facts.

use alloc::{format, string::String, vec::Vec};

use super::super::{LintFinding, LintSeverity};

/// Borrowing view over the battery fields these lints inspect.
#[derive(Debug, Clone, Copy)]
pub struct BatteryLintInput<'a> {
    pub nominal_voltage_v: f64,
    pub nominal_capacity_ah: f64,
    pub rated_energy_wh: Option<f64>,
    pub rated_capacity_kwh: Option<f64>,
    pub operating_temp_min_c: Option<f64>,
    pub operating_temp_max_c: Option<f64>,
    /// Unix seconds. `None` when the passport carries no manufacturing date.
    pub manufacturing_date_unix: Option<i64>,
    /// Unix seconds "now" — this crate has no clock, so the caller supplies it.
    pub as_of_unix: i64,
    pub cathode_material_pct: &'a [f64],
    pub anode_material_pct: &'a [f64],
    pub electrolyte_material_pct: &'a [f64],
}

const ENERGY_CAPACITY_TOLERANCE_PCT: f64 = 20.0;

/// Physics check: energy (Wh) = voltage (V) × capacity (Ah). A declared
/// `ratedEnergyWh` more than `ENERGY_CAPACITY_TOLERANCE_PCT` away from the
/// V×Ah product is more likely a data-entry error than a real spec — though a
/// generous tolerance is kept since manufacturers sometimes rate energy at
/// average (not nominal) discharge voltage, hence `Notice` not `Warning`.
#[must_use]
pub fn energy_capacity_mismatch(input: &BatteryLintInput<'_>) -> Option<LintFinding> {
    let declared = input.rated_energy_wh?;
    let computed = input.nominal_voltage_v * input.nominal_capacity_ah;
    if !declared.is_finite() || !computed.is_finite() || computed <= 0.0 {
        return None;
    }
    let deviation_pct = ((declared - computed).abs() / computed) * 100.0;
    if deviation_pct <= ENERGY_CAPACITY_TOLERANCE_PCT {
        return None;
    }
    Some(LintFinding {
        code: "battery.energy_capacity_mismatch",
        field: "ratedEnergyWh",
        severity: LintSeverity::Notice,
        message: format!(
            "ratedEnergyWh ({declared:.1} Wh) differs from nominalVoltageV × nominalCapacityAh \
             ({computed:.1} Wh) by {deviation_pct:.0}% — intended?"
        ),
    })
}

const CAPACITY_UNIT_TOLERANCE_PCT: f64 = 5.0;

/// Unit-conversion check: `ratedCapacityKwh × 1000` and `ratedEnergyWh`
/// describe the same physical quantity in different units and should match
/// closely — unlike the voltage×capacity estimate above, there is no
/// legitimate reason for these two to diverge by more than rounding.
#[must_use]
pub fn rated_capacity_kwh_wh_mismatch(input: &BatteryLintInput<'_>) -> Option<LintFinding> {
    let kwh = input.rated_capacity_kwh?;
    let wh = input.rated_energy_wh?;
    if !kwh.is_finite() || !wh.is_finite() {
        return None;
    }
    let computed_wh = kwh * 1000.0;
    if computed_wh <= 0.0 {
        return None;
    }
    let deviation_pct = ((wh - computed_wh).abs() / computed_wh) * 100.0;
    if deviation_pct <= CAPACITY_UNIT_TOLERANCE_PCT {
        return None;
    }
    Some(LintFinding {
        code: "battery.rated_capacity_kwh_wh_mismatch",
        field: "ratedCapacityKwh",
        severity: LintSeverity::Warning,
        message: format!(
            "ratedCapacityKwh ({kwh:.3} kWh = {computed_wh:.1} Wh) does not match ratedEnergyWh \
             ({wh:.1} Wh) — intended?"
        ),
    })
}

const MATERIAL_SUM_TOLERANCE_PCT: f64 = 2.0;

fn material_sum_finding(field: &'static str, pcts: &[f64]) -> Option<LintFinding> {
    if pcts.is_empty() {
        return None;
    }
    let (within_tolerance, total) =
        crate::common::numeric::sums_to(pcts.iter().copied(), 100.0, MATERIAL_SUM_TOLERANCE_PCT);
    if !total.is_finite() || within_tolerance {
        return None;
    }
    Some(LintFinding {
        code: "battery.material_composition_sum",
        field,
        severity: LintSeverity::Warning,
        message: format!(
            "{field} weightPct entries sum to {total:.1}%, expected ~100% — intended?"
        ),
    })
}

/// Sum-consistency check: `cathodeMaterial`, `anodeMaterial`, and
/// `electrolyteMaterial` are each declared as a `weightPct` breakdown of that
/// component — none of the three currently have a hard schema or cross-field
/// gate requiring them to sum to 100%. Fires independently per list, so 0–3
/// findings can result from one call.
#[must_use]
pub fn material_composition_sums(input: &BatteryLintInput<'_>) -> Vec<LintFinding> {
    [
        ("cathodeMaterial", input.cathode_material_pct),
        ("anodeMaterial", input.anode_material_pct),
        ("electrolyteMaterial", input.electrolyte_material_pct),
    ]
    .into_iter()
    .filter_map(|(field, pcts)| material_sum_finding(field, pcts))
    .collect()
}

/// Cross-field ordering: a declared manufacturing date after "now" cannot be
/// correct — the battery hasn't been made yet.
#[must_use]
pub fn manufacturing_date_in_future(input: &BatteryLintInput<'_>) -> Option<LintFinding> {
    let mfg = input.manufacturing_date_unix?;
    if mfg <= input.as_of_unix {
        return None;
    }
    Some(LintFinding {
        code: "battery.manufacturing_date_in_future",
        field: "manufacturingDate",
        severity: LintSeverity::Warning,
        message: String::from("manufacturingDate is in the future — intended?"),
    })
}

const OPERATING_TEMP_MIN_PLAUSIBLE_C: f64 = -60.0;
const OPERATING_TEMP_MAX_PLAUSIBLE_C: f64 = 150.0;

/// Range plausibility: no commercial battery chemistry operates outside
/// roughly -60°C to 150°C. A declared bound beyond that is far more likely a
/// unit slip (e.g. Fahrenheit entered as Celsius) than a real spec. Distinct
/// from [`crate::batteries::chemistry::validate_operating_temp_range`], which
/// checks `min < max` — this checks each bound against a plausible envelope.
#[must_use]
pub fn operating_temp_absurd_range(input: &BatteryLintInput<'_>) -> Vec<LintFinding> {
    let mut out = Vec::new();
    if let Some(min) = input.operating_temp_min_c
        && min.is_finite()
        && min < OPERATING_TEMP_MIN_PLAUSIBLE_C
    {
        out.push(LintFinding {
            code: "battery.operating_temp_range_implausible",
            field: "operatingTempMinC",
            severity: LintSeverity::Notice,
            message: format!(
                "operatingTempMinC ({min}°C) is outside the plausible range for any known battery \
                 chemistry — intended?"
            ),
        });
    }
    if let Some(max) = input.operating_temp_max_c
        && max.is_finite()
        && max > OPERATING_TEMP_MAX_PLAUSIBLE_C
    {
        out.push(LintFinding {
            code: "battery.operating_temp_range_implausible",
            field: "operatingTempMaxC",
            severity: LintSeverity::Notice,
            message: format!(
                "operatingTempMaxC ({max}°C) is outside the plausible range for any known battery \
                 chemistry — intended?"
            ),
        });
    }
    out
}

/// Run every battery plausibility lint and collect the findings.
#[must_use]
pub fn lint_battery(input: &BatteryLintInput<'_>) -> Vec<LintFinding> {
    let mut out = Vec::new();
    out.extend(energy_capacity_mismatch(input));
    out.extend(rated_capacity_kwh_wh_mismatch(input));
    out.extend(material_composition_sums(input));
    out.extend(manufacturing_date_in_future(input));
    out.extend(operating_temp_absurd_range(input));
    out
}
