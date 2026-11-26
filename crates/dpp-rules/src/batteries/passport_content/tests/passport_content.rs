//! Tests of the `passport_content` module.

use super::super::super::battery_facts::BatteryFacts;
use super::super::super::due_diligence::{
    DUE_DILIGENCE_REQUIRED_FROM, DueDiligenceSecondLife, OperatorScope,
};
use super::super::*;
use crate::common::date::CalendarDate;

/// A record that has said nothing a scheduled row reads.
const NO_FACTS: BatteryFacts = BatteryFacts {
    placed_on_market: None,
    second_life: DueDiligenceSecondLife::None,
    operator_scope: None,
};

/// An original battery placed on the first day of the passport duty, before
/// row 19 moves.
const PLACED_FEBRUARY_2027: BatteryFacts = BatteryFacts {
    placed_on_market: Some(CalendarDate::new(2027, 2, 18)),
    ..NO_FACTS
};

const CATEGORIES: [&str; 3] = ["ev", "lmt", "industrial"];

fn placed(date: CalendarDate, operator_scope: Option<OperatorScope>) -> BatteryFacts {
    BatteryFacts {
        placed_on_market: Some(date),
        operator_scope,
        ..NO_FACTS
    }
}

#[test]
fn point_1_k_is_ev_only() {
    // The sharpest per-category split in the guidance: mandatory for EV,
    // "not to be filled/displayed" for the other two.
    let f = "capacityThresholdForExhaustionPct";
    assert_eq!(
        annex_xiii_requirement(f, "ev", NO_FACTS),
        Requirement::Mandatory
    );
    assert_eq!(
        annex_xiii_requirement(f, "lmt", NO_FACTS),
        Requirement::NotApplicable
    );
    assert_eq!(
        annex_xiii_requirement(f, "industrial", NO_FACTS),
        Requirement::NotApplicable
    );
}

#[test]
fn cycle_lifetime_is_conditional_for_industrial_only() {
    for f in ["expectedLifetimeCycles", "expectedLifetimeReferenceTest"] {
        assert_eq!(
            annex_xiii_requirement(f, "ev", NO_FACTS),
            Requirement::Mandatory
        );
        assert_eq!(
            annex_xiii_requirement(f, "lmt", NO_FACTS),
            Requirement::Mandatory
        );
        assert_eq!(
            annex_xiii_requirement(f, "industrial", NO_FACTS),
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
        annex_xiii_requirement("ratedCapacityAh", "ev", NO_FACTS),
        Requirement::NotApplicable
    );
    assert_eq!(
        annex_xiii_requirement("nominalCapacityAh", "ev", NO_FACTS),
        Requirement::Mandatory
    );
}

/// The fields the guidance defers are barred for every category, and the
/// deferral is not the same thing as a disclosure class.
///
/// Guidance rows 18 to 23 read *"not to be filled/displayed as of February
/// 2027"*: the carbon footprint label, the due diligence report, and the four
/// recycled-content shares, which v1.0 had mandatory. `co2ePerUnitKg` is
/// barred beside row 17, the declaration. `dueDiligenceUrl` is
/// simultaneously an Annex XIII point 1(d) field and therefore **public**
/// once it is filled; the two facts sit on different axes and neither cancels
/// the other. A reader who conflates them will either publish a value the act
/// has not defined, or withhold one the annex puts in the public tier.
///
/// Asked of a battery placed in February 2027, because row 19 moves in
/// August: its answer for a later battery is
/// `row_19_follows_the_placing_date_in_every_category`'s question.
#[test]
fn the_deferred_data_points_are_barred_for_every_category() {
    for field in [
        "co2ePerUnitKg",
        "carbonFootprintClass",
        "dueDiligenceUrl",
        "recycledContentCobaltPct",
        "recycledContentLithiumPct",
        "recycledContentNickelPct",
        "recycledContentLeadPct",
    ] {
        for category in CATEGORIES {
            assert_eq!(
                annex_xiii_requirement(field, category, PLACED_FEBRUARY_2027),
                Requirement::NotApplicable,
                "{field} is deferred by the guidance for {category}"
            );
        }
        assert!(
            !annex_xiii_requirement(field, "ev", PLACED_FEBRUARY_2027).permits_presence(),
            "{field} must not be carried while the format is unspecified"
        );
    }

    // And they are reported by the helper the linter uses, rather than only
    // being absent from the mandatory set. No `Vec` here — this crate is
    // `no_std`.
    // 🚨 The control is a field the table does not name, and the reason it
    // is not flagged is [`Requirement::Unknown`] — **not** that it is
    // mandatory. Only `NotApplicable` is barred, so anything the guidance
    // has not been read against passes here, and reading that as approval
    // is the mistake this control exists to make visible.
    //
    // It used to be `"gtin"`, which was doubly misleading: the assertion
    // claimed the table answered `Mandatory` for it (the table has no such
    // row — the identifier is the envelope's carrier identifier, which this
    // table does not check), and product group data stopped carrying a bare
    // `gtin` at all.
    let present = [
        "productIdentifier",
        "dueDiligenceUrl",
        "carbonFootprintClass",
    ];
    let mut flagged = fields_not_applicable(&present, "ev", PLACED_FEBRUARY_2027);
    assert_eq!(flagged.next(), Some("dueDiligenceUrl"));
    assert_eq!(flagged.next(), Some("carbonFootprintClass"));
    assert_eq!(
        flagged.next(),
        None,
        "productIdentifier is Unknown to this table, and Unknown is not barred"
    );
    assert_eq!(
        annex_xiii_requirement("productIdentifier", "ev", NO_FACTS),
        Requirement::Unknown,
        "if the guidance is ever read against the product identifier, this \
             control stops being one and the line above needs rewriting"
    );
}

#[test]
fn portable_and_sli_are_unknown_not_exempt() {
    // The guidance covers three categories. Answering "not applicable" for
    // the other two would turn an absence of evidence into a finding.
    for t in ["portable", "starting-lighting-ignition", "sli", ""] {
        assert_eq!(
            annex_xiii_requirement("batteryType", t, NO_FACTS),
            Requirement::Unknown,
            "{t} is outside the guidance's scope"
        );
    }
}

#[test]
fn an_unnamed_field_is_unknown() {
    assert_eq!(
        annex_xiii_requirement("somethingNobodyHasChecked", "ev", NO_FACTS),
        Requirement::Unknown
    );
}

#[test]
fn category_matching_ignores_case_and_padding() {
    assert_eq!(
        annex_xiii_requirement("batteryType", "  EV  ", NO_FACTS),
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
        Requirement::Exempt,
        Requirement::Undetermined(MissingFact::PlacingDate),
    ] {
        assert!(r.permits_presence());
    }
}

#[test]
fn row_19_follows_the_placing_date_in_every_category() {
    let day_before = CalendarDate::new(2027, 8, 17);
    let subject = Some(OperatorScope::Subject);
    for category in CATEGORIES {
        let ask = |facts| annex_xiii_requirement("dueDiligenceUrl", category, facts);
        assert_eq!(
            ask(placed(day_before, subject)),
            Requirement::NotApplicable,
            "{category}: placed before 18 August 2027"
        );
        assert_eq!(
            ask(placed(DUE_DILIGENCE_REQUIRED_FROM, subject)),
            Requirement::Mandatory,
            "{category}: placed on 18 August 2027 by an operator subject to Chapter VII"
        );
        assert_eq!(
            ask(NO_FACTS),
            Requirement::Undetermined(MissingFact::PlacingDate),
            "{category}: no date"
        );
    }
}

#[test]
fn row_19_honours_both_art_47_exemptions() {
    for category in CATEGORIES {
        let ask = |facts| annex_xiii_requirement("dueDiligenceUrl", category, facts);
        assert_eq!(
            ask(placed(DUE_DILIGENCE_REQUIRED_FROM, None)),
            Requirement::Undetermined(MissingFact::OperatorScope),
            "{category}: an undeclared operator is asked, not excused"
        );
        assert_eq!(
            ask(placed(
                DUE_DILIGENCE_REQUIRED_FROM,
                Some(OperatorScope::BelowTurnoverThreshold)
            )),
            Requirement::Exempt,
            "{category}: Art. 47, first paragraph"
        );
        let second_life = BatteryFacts {
            second_life: DueDiligenceSecondLife::PlacedBeforeOperations,
            ..NO_FACTS
        };
        assert_eq!(
            ask(second_life),
            Requirement::Exempt,
            "{category}: Art. 47, second paragraph, needs no date"
        );
    }
}

#[test]
fn a_row_with_nothing_scheduled_ignores_the_facts() {
    // A missing date must cost exactly the rows that need one.
    let on_the_date = placed(DUE_DILIGENCE_REQUIRED_FROM, Some(OperatorScope::Subject));
    for field in [
        "batteryType",
        "capacityThresholdForExhaustionPct",
        "carbonFootprintClass",
        "expectedLifetimeCycles",
    ] {
        for category in CATEGORIES {
            assert_eq!(
                annex_xiii_requirement(field, category, NO_FACTS),
                annex_xiii_requirement(field, category, on_the_date),
                "{field} for {category}"
            );
        }
    }
}

#[test]
fn portable_and_sli_stay_unknown_whatever_the_date() {
    let on_the_date = placed(DUE_DILIGENCE_REQUIRED_FROM, Some(OperatorScope::Subject));
    for t in ["portable", "starting-lighting-ignition"] {
        for facts in [NO_FACTS, PLACED_FEBRUARY_2027, on_the_date] {
            assert_eq!(
                annex_xiii_requirement("dueDiligenceUrl", t, facts),
                Requirement::Unknown,
                "{t} {facts:?}"
            );
        }
        assert_eq!(undetermined_fields(t, NO_FACTS).count(), 0, "{t}");
    }
}

#[test]
fn the_gate_inputs_see_the_scheduled_row() {
    let on_the_date = placed(DUE_DILIGENCE_REQUIRED_FROM, Some(OperatorScope::Subject));
    assert!(mandatory_fields("ev", on_the_date).any(|f| f == "dueDiligenceUrl"));
    assert!(!mandatory_fields("ev", PLACED_FEBRUARY_2027).any(|f| f == "dueDiligenceUrl"));

    let mut waiting = undetermined_fields("lmt", NO_FACTS);
    assert_eq!(
        waiting.next(),
        Some(("dueDiligenceUrl", MissingFact::PlacingDate))
    );
    assert_eq!(waiting.next(), None, "only row 19 depends on a fact today");
    assert_eq!(undetermined_fields("lmt", PLACED_FEBRUARY_2027).count(), 0);

    // An exempt battery may still carry the address; it is not flagged.
    let exempt = placed(
        DUE_DILIGENCE_REQUIRED_FROM,
        Some(OperatorScope::BelowTurnoverThreshold),
    );
    let present = ["dueDiligenceUrl"];
    assert_eq!(fields_not_applicable(&present, "ev", exempt).count(), 0);
    assert!(!mandatory_fields("ev", exempt).any(|f| f == "dueDiligenceUrl"));
}

#[test]
fn a_missing_fact_names_the_field_that_supplies_it() {
    assert_eq!(MissingFact::PlacingDate.field(), "placedOnMarketDate");
    assert_eq!(
        MissingFact::OperatorScope.field(),
        "dueDiligenceOperatorScope"
    );
}
