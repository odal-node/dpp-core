//! The advisory findings Art. 8 produces for one battery: a share declared for
//! a metal its chemistry does not contain, shares without a reporting year, and
//! shortfalls against whichever phase binds.

use dpp_plugin_sdk::rules::batteries::recycled_content::{
    Art8Category, Art8Phase, RecycledContentInput, art8_category_for, art8_phase_for,
    art8_shortfalls_2031, art8_shortfalls_2036, chemistry_regulated_metals,
    recycled_content_chemistry_conflicts,
};
use dpp_plugin_sdk::rules::common::date::CalendarDate;
use dpp_plugin_sdk::traits::{PluginFinding, PluginInput};
use dpp_plugin_sdk::validate::num;
use serde_json::Value;

use super::second_life::art8_second_life;
use super::shares::share_pointer;

/// Every Art. 8 finding for this battery, in the order they are reported.
///
/// All of them are advisory. The plugin never gates on Art. 8, and the overall
/// status stays `NotAssessed`.
pub(crate) fn recycled_content_findings(
    input: &PluginInput,
    battery_type: &str,
    chemistry: &str,
    declared: RecycledContentInput,
) -> Vec<PluginFinding> {
    let RecycledContentInput {
        cobalt_pct: cobalt,
        lithium_pct: lithium,
        nickel_pct: nickel,
        lead_pct: lead,
    } = declared;
    let mut warnings: Vec<PluginFinding> = Vec::new();

    let regulated = chemistry_regulated_metals(chemistry);
    // Energy capacity for the Phase-1 ">2 kWh" scoping threshold: the rated
    // value if given, else nominal V × Ah (both required, so always known).
    let capacity_kwh = num(input, "ratedCapacityKwh").or_else(|| {
        match (
            num(input, "nominalVoltageV"),
            num(input, "nominalCapacityAh"),
        ) {
            (Some(v), Some(ah)) => Some(v * ah / 1000.0),
            _ => None,
        }
    });

    // Data integrity: recycled content declared for a metal the chemistry
    // does not contain (e.g. cobalt on an LFP cell) is a contradiction —
    // surface it as an advisory rather than folding it into the mean silently.
    for metal in recycled_content_chemistry_conflicts(chemistry, cobalt, lithium, nickel, lead) {
        warnings.push(PluginFinding::new(
            format!("battery.recycled_content.{metal}_not_in_chemistry"),
            share_pointer(metal),
            format!(
                "Declared {metal} recycled content, but a {chemistry} battery contains \
                 no {metal} — check the declaration."
            ),
        ));
    }

    // Recycled-content targets (EU 2023/1542 Art. 8(2)/(3)). Which phase
    // binds is decided by the date the battery was placed on the EU market,
    // never by today's date: Art. 8 attaches its duties at placing on the
    // market, so a battery lawfully placed in 2030 never acquires the 2031
    // minima. Shortfalls are scoped to the metals the declared chemistry
    // actually contains (an LFP cell has no cobalt or nickel), and are always
    // advisory — the overall status is NotAssessed.
    let art8_category = art8_category_for(battery_type, capacity_kwh);

    // Art. 8(1) requires the shares "for each battery model per year and per
    // manufacturing plant". A percentage without both anchors is not that
    // declaration — it is an unattributed number, and no reader can tell
    // which production run it describes.
    let declared_any_share = [cobalt, lithium, nickel, lead].iter().any(Option::is_some);
    if declared_any_share
        && art8_category != Art8Category::NotCovered
        && input.get("recycledContentReportingYear").is_none()
    {
        warnings.push(PluginFinding::new(
            "battery.recycled_content.reporting_year_missing",
            "/recycledContentReportingYear",
            "Recycled-content shares are declared without a reporting year. \
             EU 2023/1542 Art. 8(1) requires them per battery model, per year \
             and per manufacturing plant, so a share without its year is not \
             the Art. 8(1) declaration.",
        ));
    }

    if art8_category != Art8Category::NotCovered {
        let scoped = RecycledContentInput {
            cobalt_pct: if regulated.cobalt { cobalt } else { None },
            lithium_pct: if regulated.lithium { lithium } else { None },
            nickel_pct: if regulated.nickel { nickel } else { None },
            lead_pct: if regulated.lead { lead } else { None },
        };
        let placed_on_market = input
            .get("placedOnMarketDate")
            .and_then(Value::as_str)
            .and_then(CalendarDate::parse_iso);

        match placed_on_market {
            // Declining to guess is the whole point. Defaulting to "today"
            // is what produces retroactively wrong findings once a phase
            // begins, so say what is missing instead.
            None => warnings.push(PluginFinding::new(
                "battery.recycled_content.market_date_missing",
                "/placedOnMarketDate",
                "Cannot determine which EU 2023/1542 Art. 8 recycled-content phase \
                 applies without a valid placedOnMarketDate (YYYY-MM-DD). The minimum \
                 shares attach by the date the battery was placed on the EU market.",
            )),
            Some(date) => {
                let (shortfalls, year, standing) =
                    match art8_phase_for(art8_category, date, art8_second_life(input)) {
                        Art8Phase::Phase1 => (
                            art8_shortfalls_2031(&scoped),
                            "2031",
                            "binding for this battery",
                        ),
                        Art8Phase::Phase2 => (
                            art8_shortfalls_2036(&scoped),
                            "2036",
                            "binding for this battery",
                        ),
                        // In scope, but placed on the market before the phase
                        // began. Reported as forward-looking guidance rather
                        // than dropped: useful to know, but not a duty this
                        // battery carries.
                        Art8Phase::NotYetBinding => match art8_category {
                            Art8Category::Lmt => (
                                art8_shortfalls_2036(&scoped),
                                "2036",
                                "not binding for this battery — Art. 8(3) applies to LMT \
                             batteries placed on the market from 18 Aug 2036",
                            ),
                            _ => (
                                art8_shortfalls_2031(&scoped),
                                "2031",
                                "not binding for this battery — Art. 8(2) applies to \
                             batteries placed on the market from 18 Aug 2031",
                            ),
                        },
                        // Art. 8(4): the paragraphs do not apply at all. No
                        // shortfall is reportable, and reporting one would
                        // assert a duty the Regulation removed.
                        //
                        // Said out loud rather than left silent. Absence of a
                        // recycled-content finding is not a statement — it is
                        // what a battery outside Art. 8, a battery whose shares
                        // are all met, and a battery nobody assessed all look
                        // like. An exemption an operator has to infer from
                        // silence is one they cannot show an authority.
                        Art8Phase::ExemptSecondLife => {
                            warnings.push(PluginFinding::new(
                                "battery.recycled_content.exempt_second_life",
                                "/batteryStatus",
                                "EU 2023/1542 Art. 8(4) disapplies the Art. 8(1)-(3) \
                             recycled-content minimums to this battery: batteryStatus \
                             records a second-life operation, and a battery reaching \
                             that state was necessarily on the market before it. No \
                             minimum share is assessed.",
                            ));
                            (Vec::new(), "", "")
                        }
                        // Nothing to say: the category was never in scope, so
                        // there is no duty whose absence needs explaining.
                        Art8Phase::NotCovered => (Vec::new(), "", ""),
                    };
                for sf in shortfalls {
                    warnings.push(PluginFinding::new(
                        format!(
                            "battery.recycled_content.{}_below_{year}_target",
                            sf.material
                        ),
                        share_pointer(sf.material),
                        format!(
                            "Declared {} recycled content {:.1}% is below the EU 2023/1542 \
                             Art. 8 minimum of {:.0}% ({standing}).",
                            sf.material, sf.declared_pct, sf.required_pct
                        ),
                    ));
                }
            }
        }
    }

    warnings
}
