//! Which battery passport data points a given battery category actually owes.
//!
//! The schema deliberately declares almost every Annex XIII field optional, and
//! that is not laxity: the obligations are **per category**. A field mandatory
//! for an electric-vehicle battery may be *"not to be filled/displayed"* for an
//! LMT one, and requiring it in JSON Schema would make a lawful industrial
//! battery unrepresentable. The constraint belongs here, where it can be
//! expressed as a function of the category.
//!
//! # Source
//!
//! The Commission's *Guidance Document: Digital Batteries Passport — data
//! points by category* (v1.0, 28 July 2026), read in full against the model.
//! Its table has one row per data point and one column per category, which is
//! exactly the shape of the `REQUIREMENTS` table below — so a reviewer can diff the two
//! directly rather than reconstructing the mapping.
//!
//! **The guidance covers EV, LMT and industrial batteries only.** It says
//! nothing about portable or SLI batteries, so this module answers
//! [`Requirement::Unknown`] for them rather than guessing. Silence in the source
//! is not permission, and it is not prohibition either.

/// What a category owes for one data point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Requirement {
    /// The guidance marks it mandatory. A published passport without it is
    /// missing content the law requires.
    Mandatory,
    /// *"If applicable"*, *"where possible"*, or *"only applicable for some
    /// industrial batteries"* — the duty exists but its trigger is a fact about
    /// the individual battery that no schema can decide.
    Conditional,
    /// *"Not to be filled/displayed"*. Present in a passport of this category,
    /// the value is not merely surplus — the guidance says it does not belong.
    NotApplicable,
    /// The guidance does not cover this category, or does not name this field.
    /// Distinct from [`Self::NotApplicable`]: one is a recorded exclusion, the
    /// other is an absence of evidence.
    Unknown,
}

impl Requirement {
    /// Whether a passport of this category may carry the field at all.
    #[must_use]
    pub const fn permits_presence(self) -> bool {
        !matches!(self, Self::NotApplicable)
    }
}

/// The category a rule is being asked about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Category {
    Ev,
    Lmt,
    Industrial,
}

fn category_of(battery_type: &str) -> Option<Category> {
    let t = battery_type.trim();
    let eq = |s: &str| t.eq_ignore_ascii_case(s);
    if eq("ev") {
        Some(Category::Ev)
    } else if eq("lmt") {
        Some(Category::Lmt)
    } else if eq("industrial") {
        Some(Category::Industrial)
    } else {
        // portable, sli / starting-lighting-ignition, unknown — outside the
        // guidance's scope entirely.
        None
    }
}

use Requirement::{Conditional as C, Mandatory as M, NotApplicable as X};

/// One row per data point: the wire field name, then EV, LMT, industrial.
///
/// Ordered to follow the Commission's own numbering, so the two can be read
/// side by side. Fields the guidance marks mandatory for all three are included
/// rather than defaulted, because "we checked and it is uniform" and "we did not
/// check" must not look the same in this table.
const REQUIREMENTS: &[(&str, Requirement, Requirement, Requirement)] = &[
    // ── Annex VI Part A, reached by Annex XIII point 1(a) ──────────────────
    ("batteryType", M, M, M),
    ("batteryWeightKg", M, M, M),
    ("nominalCapacityAh", M, M, M),
    ("batteryChemistry", M, M, M),
    ("hazardousSubstances", M, M, M),
    ("usableExtinguishingAgent", M, M, M),
    ("criticalRawMaterials", M, M, M),
    // ── Annex XIII point 1 ─────────────────────────────────────────────────
    ("recycledContentCobaltPct", M, M, M),
    ("recycledContentLithiumPct", M, M, M),
    ("recycledContentNickelPct", M, M, M),
    ("recycledContentLeadPct", M, M, M),
    ("renewableContentPct", M, M, M),
    // Point 1(g) is "not to be filled/displayed" for every category. The Ah
    // figure a passport does carry is Annex VI Part A point 6, above.
    ("ratedCapacityAh", X, X, X),
    ("minimalVoltageV", M, M, M),
    ("nominalVoltageV", M, M, M),
    ("maximumVoltageV", M, M, M),
    ("originalPowerCapabilityW", M, M, M),
    ("powerLimitMinW", M, M, M),
    ("powerLimitMaxW", M, M, M),
    // 1(j): "only applicable for some industrial batteries where lifetime can
    // be expressed in cycles".
    ("expectedLifetimeCycles", M, M, C),
    ("expectedLifetimeReferenceTest", M, M, C),
    // 1(k): mandatory for EV only, and explicitly not for the other two.
    ("capacityThresholdForExhaustionPct", M, X, X),
    ("notInUseTemperatureRange", M, M, M),
    ("notInUseTemperatureReferenceTest", M, M, M),
    // 1(m): "only if applicable (if commercial warranty envisaged)".
    ("commercialWarrantyPeriodMonths", C, C, C),
    // 1(n): "only applicable for some industrial batteries".
    ("initialRoundTripEfficiencyPct", M, M, C),
    ("roundTripEfficiencyAtHalfCycleLifePct", M, M, C),
    ("internalCellResistanceMohm", M, M, M),
    ("internalPackResistanceMohm", M, M, M),
    ("cycleLifeTestCRate", M, M, C),
    ("markingInformation", M, M, M),
    // 1(q), Art. 13(5): "cadmium or lead symbol if applicable".
    ("hazardSymbol", C, C, C),
    ("euDeclarationOfConformity", M, M, M),
    ("wasteBatteryInformation", M, M, M),
    // ── Annex XIII points 2 and 3 ──────────────────────────────────────────
    ("cathodeMaterial", M, M, M),
    ("anodeMaterial", M, M, M),
    ("electrolyteMaterial", M, M, M),
    ("componentPartNumbers", M, M, M),
    ("sparePartsContacts", M, M, M),
    ("disassemblyInstructionsUrl", M, M, M),
    ("safetyMeasures", M, M, M),
    ("testReportResults", M, M, M),
    // ── Annex XIII point 4 — the individual-battery tier ───────────────────
    // 4(a): mandatory for EV and LMT, "if applicable" for industrial.
    ("dynamicPerformance", M, M, C),
    // 4(b): the two disjoint state-of-health lists. Which parameter set applies
    // is `degradation::annex_vii_parameter_set_for`; that the block is owed at
    // all is here.
    ("stateOfHealth", M, M, C),
    ("batteryStatus", M, M, M),
    // 4(d): "if applicable" for every category.
    ("usageHistory", C, C, C),
];

/// What a battery of `battery_type` owes for the passport field `field`.
///
/// `field` is the **wire** name — `expectedLifetimeCycles`, not
/// `expected_lifetime_cycles` — because this crate is `no_std` and zero-dep and
/// is consumed both by the domain types and by the Wasm sector plugins, which
/// see JSON and never the Rust struct.
///
/// Returns [`Requirement::Unknown`] for a category the guidance does not cover
/// (portable, SLI) and for a field it does not name. A caller must not read
/// that as permission or as prohibition — it means nobody has checked.
#[must_use]
pub fn annex_xiii_requirement(field: &str, battery_type: &str) -> Requirement {
    let Some(category) = category_of(battery_type) else {
        return Requirement::Unknown;
    };
    let mut i = 0;
    while i < REQUIREMENTS.len() {
        let (name, ev, lmt, ind) = REQUIREMENTS[i];
        if name.as_bytes() == field.as_bytes() {
            return match category {
                Category::Ev => ev,
                Category::Lmt => lmt,
                Category::Industrial => ind,
            };
        }
        i += 1;
    }
    Requirement::Unknown
}

/// Every field this category must carry, in table order.
///
/// The publish gate's input. Iterating the table rather than exposing it keeps
/// the rows private, so a caller cannot come to depend on their order or arity.
pub fn mandatory_fields(battery_type: &str) -> impl Iterator<Item = &'static str> {
    let category = category_of(battery_type);
    REQUIREMENTS.iter().filter_map(move |(name, ev, lmt, ind)| {
        let r = match category? {
            Category::Ev => *ev,
            Category::Lmt => *lmt,
            Category::Industrial => *ind,
        };
        (r == Requirement::Mandatory).then_some(*name)
    })
}

/// Every field of `present` that this category must not carry.
///
/// The complement of the usual question. A passport asserting a capacity
/// threshold for exhaustion on an LMT battery is not missing anything — it is
/// carrying content the guidance says does not belong, which a
/// mandatory-fields check would never notice.
pub fn fields_not_applicable<'a>(
    present: &'a [&'a str],
    battery_type: &str,
) -> impl Iterator<Item = &'a str> {
    present
        .iter()
        .copied()
        .filter(move |f| annex_xiii_requirement(f, battery_type) == Requirement::NotApplicable)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_1_k_is_ev_only() {
        // The sharpest per-category split in the guidance: mandatory for EV,
        // "not to be filled/displayed" for the other two.
        let f = "capacityThresholdForExhaustionPct";
        assert_eq!(annex_xiii_requirement(f, "ev"), Requirement::Mandatory);
        assert_eq!(annex_xiii_requirement(f, "lmt"), Requirement::NotApplicable);
        assert_eq!(
            annex_xiii_requirement(f, "industrial"),
            Requirement::NotApplicable
        );
    }

    #[test]
    fn cycle_lifetime_is_conditional_for_industrial_only() {
        for f in ["expectedLifetimeCycles", "expectedLifetimeReferenceTest"] {
            assert_eq!(annex_xiii_requirement(f, "ev"), Requirement::Mandatory);
            assert_eq!(annex_xiii_requirement(f, "lmt"), Requirement::Mandatory);
            assert_eq!(
                annex_xiii_requirement(f, "industrial"),
                Requirement::Conditional,
                "{f}: 'only applicable for some industrial batteries where \
                 lifetime can be expressed in cycles'"
            );
        }
    }

    #[test]
    fn the_suppressed_ah_figure_is_not_the_mandatory_one() {
        // Point 1(g) is suppressed for every category while Annex VI Part A
        // point 6 is mandatory for every category. Two data points, one
        // quantity — the pair this table exists to keep apart.
        assert_eq!(
            annex_xiii_requirement("ratedCapacityAh", "ev"),
            Requirement::NotApplicable
        );
        assert_eq!(
            annex_xiii_requirement("nominalCapacityAh", "ev"),
            Requirement::Mandatory
        );
    }

    #[test]
    fn portable_and_sli_are_unknown_not_exempt() {
        // The guidance covers three categories. Answering "not applicable" for
        // the other two would turn an absence of evidence into a finding.
        for t in ["portable", "starting-lighting-ignition", "sli", ""] {
            assert_eq!(
                annex_xiii_requirement("batteryType", t),
                Requirement::Unknown,
                "{t} is outside the guidance's scope"
            );
        }
    }

    #[test]
    fn an_unnamed_field_is_unknown() {
        assert_eq!(
            annex_xiii_requirement("somethingNobodyHasChecked", "ev"),
            Requirement::Unknown
        );
    }

    #[test]
    fn category_matching_ignores_case_and_padding() {
        assert_eq!(
            annex_xiii_requirement("batteryType", "  EV  "),
            Requirement::Mandatory
        );
    }

    #[test]
    fn not_applicable_is_the_only_verdict_that_bars_presence() {
        assert!(!Requirement::NotApplicable.permits_presence());
        for r in [
            Requirement::Mandatory,
            Requirement::Conditional,
            Requirement::Unknown,
        ] {
            assert!(r.permits_presence());
        }
    }
}
