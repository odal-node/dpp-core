//! Battery degradation rules — performance, durability and state-of-health.
//!
//! Two separate regimes, on two separate timelines. Conflating them is the easy
//! mistake here, because only one of them is pending.
//!
//! ## Already in force — declaration duties
//!
//! - **Art. 10(1), since 18 Aug 2024**: rechargeable industrial batteries above
//!   2 kWh, LMT batteries and EV batteries must be accompanied by a document
//!   carrying the electrochemical performance and durability parameters of
//!   **Annex IV Part A**, with Annex IV Part B explaining how they were measured.
//! - **Art. 14(1), since 18 Aug 2024**: the parameters of **Annex VII** must be
//!   held in the battery management system of stationary battery energy storage
//!   systems, LMT batteries and EV batteries.
//!
//! Neither is a threshold. Both are "declare the value", and both are live today.
//!
//! ## Pending — minimum values
//!
//! The values below which a battery is non-compliant come from a delegated act
//! under **Art. 10(5)**, which has not been adopted. There are two of them:
//!
//! | Scope | Act due | Minimum values apply from |
//! |---|---|---|
//! | Industrial > 2 kWh (excl. exclusively external storage) | 18 Feb 2026 | 18 Aug 2027 |
//! | LMT batteries | 18 Feb 2027 | 18 Aug 2028 |
//!
//! Both application dates are conditional: Art. 10(2) and 10(3) read "or 18
//! months after the date of entry into force of the delegated act …, whichever
//! is the latest". Until the act enters into force the effective date is
//! unknown, not merely future.
//!
//! Art. 10(6) is a different power — it lets the Commission *amend the Annex IV
//! parameter list* in light of technical progress. It does not set minimum
//! values, and an earlier version of this module cited it in their place.
//!
//! ## Art. 10(4) — the second-life carve-out
//!
//! Art. 10(1)–(3) do not apply to a battery prepared for re-use, prepared for
//! repurposing, repurposed or remanufactured, where the operator demonstrates it
//! was placed on the market **before** those obligations became applicable. A
//! determination therefore depends on the *original* placing-on-market date, not
//! the date of assessment.
//!
//! ## What Annex VII actually asks for
//!
//! State of health is a parameter set that differs by battery type, not a single
//! percentage:
//!
//! - **EV batteries** — state of certified energy (SOCE).
//! - **Stationary storage and LMT** — remaining capacity; where possible
//!   remaining power capability; where possible remaining round trip efficiency;
//!   evolution of self-discharging rates; where possible ohmic resistance.
//!
//! Annex VII Part B lists the expected-lifetime parameters separately, including
//! date of manufacture and (where appropriate) date of putting into service,
//! energy throughput, capacity throughput, tracking of harmful events, and the
//! number of full equivalent charge-discharge cycles.
//!
//! Which parameter set a battery must report is decided by its category — see
//! [`annex_vii_parameter_set_for`]. The *minimum values* those parameters must
//! reach remain pending under Art. 10(5).

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ev_batteries_report_soce_alone() {
        assert_eq!(
            annex_vii_parameter_set_for("ev"),
            Some(Annex7ParameterSet::ElectricVehicle)
        );
        assert_eq!(
            annex_vii_parameter_set_for("EV"),
            Some(Annex7ParameterSet::ElectricVehicle)
        );
    }

    #[test]
    fn lmt_and_industrial_share_the_five_parameter_list() {
        for t in ["lmt", "LMT", "industrial", " Industrial "] {
            assert_eq!(
                annex_vii_parameter_set_for(t),
                Some(Annex7ParameterSet::StationaryOrLmt),
                "{t} should use the stationary/LMT list"
            );
        }
    }

    #[test]
    fn portable_and_sli_are_outside_annex_vii_part_a() {
        // Art. 14(1) names stationary storage, LMT and EV batteries only.
        for t in [
            "portable",
            "sli",
            "starting-lighting-ignition",
            "",
            "mystery",
        ] {
            assert_eq!(
                annex_vii_parameter_set_for(t),
                None,
                "{t} should be out of scope"
            );
        }
    }

    #[test]
    fn part_b_excludes_electric_vehicle_batteries() {
        // The asymmetry between the two parts. Part A names EV batteries;
        // Part B does not. An EV battery reports SOCE and no expected-lifetime
        // parameter set at all.
        assert_eq!(
            annex_vii_parameter_set_for("ev"),
            Some(Annex7ParameterSet::ElectricVehicle)
        );
        assert!(!annex_vii_part_b_applies_to("ev"));
    }

    #[test]
    fn part_b_covers_stationary_and_lmt() {
        for t in ["lmt", "industrial", "LMT"] {
            assert!(annex_vii_part_b_applies_to(t), "{t} should be in Part B");
        }
        for t in ["portable", "sli", "starting-lighting-ignition", "mystery"] {
            assert!(
                !annex_vii_part_b_applies_to(t),
                "{t} should be outside Part B"
            );
        }
    }

    #[test]
    fn the_two_parameter_sets_are_distinct() {
        assert_ne!(
            annex_vii_parameter_set_for("ev"),
            annex_vii_parameter_set_for("lmt")
        );
    }
}
