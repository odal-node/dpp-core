//! Battery chemistry content limits and the dates they apply from.

use alloc::{format, string::String};

use crate::batteries::schedule::{ChangeKind, ScheduledChange};
use crate::common::date::CalendarDate;

// ── Annex I thresholds ────────────────────────────────────────────────────────

/// Maximum mercury content (% by weight) for any battery type.
///
/// Annex I entry 1, under Art. 6(1).
pub const MERCURY_MAX_CONTENT_PCT: f64 = 0.0005;

/// Maximum cadmium content (% by weight) for **portable** batteries.
///
/// Annex I entry 2, under Art. 6(1). Industrial and EV batteries have no
/// cadmium weight-percentage limit.
pub const CADMIUM_PORTABLE_MAX_CONTENT_PCT: f64 = 0.002;

/// Maximum lead content (% by weight) for **portable** batteries.
///
/// Annex I entry 3, under Art. 6(1).
pub const LEAD_PORTABLE_MAX_CONTENT_PCT: f64 = 0.01;

/// First day the Annex I lead restriction applies — 18 August 2024.
pub const LEAD_PORTABLE_RESTRICTION_FROM: CalendarDate = CalendarDate::new(2024, 8, 18);

/// The day the portable zinc-air button cell carve-out ends — 18 August 2028.
///
/// Annex I entry 3 point 2 disapplies the lead restriction to those cells
/// *"until"* this date, so it binds them **from** it.
pub const LEAD_ZINC_AIR_BUTTON_CELL_BOUND_FROM: CalendarDate = CalendarDate::new(2028, 8, 18);

/// The changes [`LEAD_PORTABLE_RESTRICTION_FROM`] and
/// [`LEAD_ZINC_AIR_BUTTON_CELL_BOUND_FROM`] schedule.
///
/// ✅ COMPLIANCE-PIN: these two dates only. Annex I entry 3 points 1 and 2 of
/// Regulation (EU) 2023/1542, read on 2026-10-21 in the table that Regulation
/// (EU) 2026/1738 Art. 53 and Annex XII substitute for Annex I from
/// 13 August 2026 (OJ L, 24.7.2026): both points keep the dates quoted in this
/// module's documentation. ⚠️ That table also adds cadmium and lead limits for
/// electric-vehicle and SLI batteries in M1 and N1 vehicles, which the rest of
/// this module has not been read against.
pub const SCHEDULE: &[ScheduledChange] = &[
    ScheduledChange {
        id: "battery.lead.portable",
        date: LEAD_PORTABLE_RESTRICTION_FROM,
        kind: ChangeKind::Restriction,
        citation: "Regulation (EU) 2023/1542 Annex I, entry 3, point 1",
        summary: "Portable batteries placed on the market from this date may contain no \
                  more than 0,01 % lead by weight.",
        product_group: "battery",
        categories: &["portable"],
    },
    ScheduledChange {
        id: "battery.lead.zinc-air-button-cells",
        date: LEAD_ZINC_AIR_BUTTON_CELL_BOUND_FROM,
        kind: ChangeKind::Restriction,
        citation: "Regulation (EU) 2023/1542 Annex I, entry 3, point 2",
        summary: "The carve-out for portable zinc-air button cells ends, and the portable \
                  lead limit binds them for cells placed on the market from this date.",
        product_group: "battery",
        categories: &["portable"],
    },
];

/// Whether a declared mercury content percentage violates the Annex I entry 1
/// prohibition. Returns `true` (prohibited) when `content_pct > 0.0005`.
#[must_use]
pub fn mercury_content_prohibited(content_pct: f64) -> bool {
    content_pct > MERCURY_MAX_CONTENT_PCT
}

/// Whether a declared cadmium content percentage violates the Annex I entry 2
/// prohibition for **portable** batteries. Returns `true` (prohibited) when
/// `content_pct > 0.002`.
///
/// Do not call this for industrial or EV batteries — the prohibition does not apply.
#[must_use]
pub fn cadmium_content_prohibited_for_portable(content_pct: f64) -> bool {
    content_pct > CADMIUM_PORTABLE_MAX_CONTENT_PCT
}

/// Whether a declared lead content percentage violates the Annex I entry 3
/// prohibition for **portable** batteries.
///
/// Unlike mercury and cadmium this one is dated, so it needs the date the
/// battery was placed on the EU market. A battery placed before 18 August 2024
/// is not in breach however much lead it declares, and reporting one as
/// prohibited would misstate an operator's position exactly as
/// [`super::super::recycled_content::art8_phase_for`] refuses to for Art. 8.
///
/// `zinc_air_button_cell` moves the binding date to 18 August 2028 under Annex I
/// entry 3 point 2. It is a separate argument rather than a battery type because
/// the carve-out is about the cell chemistry and form, which the type enum does
/// not record.
///
/// Do not call this for industrial or EV batteries — entry 3 names portable
/// batteries only.
#[must_use]
pub fn lead_content_prohibited_for_portable(
    content_pct: f64,
    placed_on_market: CalendarDate,
    zinc_air_button_cell: bool,
) -> bool {
    let binds_from = if zinc_air_button_cell {
        LEAD_ZINC_AIR_BUTTON_CELL_BOUND_FROM
    } else {
        LEAD_PORTABLE_RESTRICTION_FROM
    };
    placed_on_market >= binds_from && content_pct > LEAD_PORTABLE_MAX_CONTENT_PCT
}

// ── Cross-field: operating temperature range ──────────────────────────────────

/// Validate that the declared operating temperature range is physically coherent:
/// `operatingTempMinC` must be strictly less than `operatingTempMaxC`.
///
/// Both fields are optional in the battery schema; this rule fires only when both
/// are present. A single absent field is not an error here.
pub fn validate_operating_temp_range(min_c: Option<f64>, max_c: Option<f64>) -> Result<(), String> {
    if let (Some(min), Some(max)) = (min_c, max_c)
        && min >= max
    {
        return Err(format!(
            "operatingTempMinC ({min}°C) must be less than operatingTempMaxC ({max}°C)"
        ));
    }
    Ok(())
}
