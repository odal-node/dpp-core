//! The degradation parameter set a battery type falls under.

/// Which Annex VII Part A parameter set applies to a battery.
///
/// Annex VII Part A splits into exactly two lists: one parameter for electric
/// vehicle batteries, and a five-parameter list shared by stationary battery
/// energy storage systems and LMT batteries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Annex7ParameterSet {
    /// Electric-vehicle batteries — state of certified energy (SOCE), and
    /// nothing else.
    ElectricVehicle,
    /// Stationary battery energy storage systems and LMT batteries — remaining
    /// capacity and the evolution of self-discharging rates unconditionally,
    /// plus remaining power capability, remaining round trip efficiency and
    /// ohmic resistance "where possible".
    StationaryOrLmt,
}

/// The Annex VII Part A parameter set a battery of `battery_type` must report,
/// or `None` where Annex VII Part A does not reach it.
///
/// Matching is case-insensitive. Portable and SLI batteries return `None`:
/// Art. 14(1) names only stationary battery energy storage systems, LMT
/// batteries and electric vehicle batteries.
///
/// **Known imprecision.** Art. 3 defines a stationary battery energy storage
/// system as a *subset* of industrial batteries, and the passport's
/// `batteryType` cannot distinguish the two. `"industrial"` therefore maps to
/// [`Annex7ParameterSet::StationaryOrLmt`], which is the conservative
/// direction: an industrial battery that is not a stationary storage system is
/// outside Annex VII Part A entirely, so the only cost is that declaring the
/// parameters is treated as expected rather than as surplus.
#[must_use]
pub fn annex_vii_parameter_set_for(battery_type: &str) -> Option<Annex7ParameterSet> {
    let t = battery_type.trim();
    let eq = |s: &str| t.eq_ignore_ascii_case(s);
    if eq("ev") {
        Some(Annex7ParameterSet::ElectricVehicle)
    } else if eq("lmt") || eq("industrial") {
        Some(Annex7ParameterSet::StationaryOrLmt)
    } else {
        // portable, sli / starting-lighting-ignition, unknown.
        None
    }
}

/// Whether Annex VII **Part B** (expected-lifetime parameters) reaches a battery
/// of `battery_type`.
///
/// Part B is narrower than Part A, and the difference is easy to miss: Part A
/// names *"electric vehicle batteries, stationary battery energy storage systems
/// and LMT batteries"*, while Part B names only *"stationary battery energy
/// storage systems and LMT batteries"*. **Electric-vehicle batteries report a
/// state of health under Part A but no expected-lifetime parameter set under
/// Part B.**
///
/// The `"industrial"` imprecision described on [`annex_vii_parameter_set_for`]
/// applies here too.
#[must_use]
pub fn annex_vii_part_b_applies_to(battery_type: &str) -> bool {
    matches!(
        annex_vii_parameter_set_for(battery_type),
        Some(Annex7ParameterSet::StationaryOrLmt)
    )
}
