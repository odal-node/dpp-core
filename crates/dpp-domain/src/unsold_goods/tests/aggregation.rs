//! A financial year's records, aggregated into the lines of the disclosure.

use chrono::NaiveDate;

use crate::identifier::CommodityCode;
use crate::test_support::sample_discard_record;
use crate::unsold_goods::{
    CnCategory, DiscardReason, DiscardedProductLine, DiscardedQuantity, FinancialYear,
};
use crate::unsold_goods::{
    DiscardRecord, DiscardedWeight, ExcludedRecord, ExclusionReason, WasteTreatmentOperation,
    aggregate,
};

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).expect("valid date")
}

fn year() -> FinancialYear {
    FinancialYear {
        start: date(2027, 1, 1),
        end: date(2027, 12, 31),
    }
}

fn record() -> DiscardRecord {
    sample_discard_record()
}

fn lines(records: &[DiscardRecord]) -> Vec<DiscardedProductLine> {
    aggregate(records, &year()).lines
}

fn category(code: &str) -> CnCategory {
    CnCategory::parse(code).expect("valid category")
}

#[test]
fn an_empty_year_aggregates_to_nothing() {
    let aggregation = aggregate(&[], &year());
    assert!(aggregation.lines.is_empty());
    assert!(aggregation.excluded.is_empty());
}

#[test]
fn records_of_one_category_and_reason_make_one_line() {
    let lines = lines(&[record(), record()]);

    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].cn_categories, [category("62")]);
    assert_eq!(lines[0].units_discarded.value, 240);
    assert_eq!(lines[0].weight_kg.value, 72);
    assert_eq!(lines[0].reason, Some(DiscardReason::DamagedOrContaminated));
}

/// Annex I, Section 2: figures are rounded to the nearest whole number — once, on
/// the line. Rounding each 400 g record to whole kilogrammes first would total 0.
#[test]
fn a_weight_is_rounded_once_after_summing_grams() {
    let parcel = DiscardRecord {
        weight: DiscardedWeight::measured(400),
        ..record()
    };
    assert_eq!(
        lines(&[parcel.clone(), parcel.clone(), parcel])
            .first()
            .unwrap()
            .weight_kg
            .value,
        1
    );
}

#[test]
fn half_a_kilogramme_rounds_up() {
    let half = DiscardRecord {
        weight: DiscardedWeight::measured(500),
        ..record()
    };
    assert_eq!(lines(&[half]).first().unwrap().weight_kg.value, 1);
}

/// Notes (f) and (g): where an estimate is used it is marked, and a total with an
/// estimate in it is an estimate.
#[test]
fn a_line_is_estimated_when_any_contributing_record_is() {
    let counted = record();
    let estimated_units = DiscardRecord {
        units: DiscardedQuantity::estimated(30),
        ..record()
    };
    let estimated_weight = DiscardRecord {
        weight: DiscardedWeight::estimated(9_000),
        ..record()
    };

    let units_only = &lines(&[counted.clone(), estimated_units])[0];
    assert!(units_only.units_discarded.estimated);
    assert!(!units_only.weight_kg.estimated);

    let weight_only = &lines(&[counted, estimated_weight])[0];
    assert!(!weight_only.units_discarded.estimated);
    assert!(weight_only.weight_kg.estimated);
}

#[test]
fn a_line_is_measured_when_every_contributing_record_is() {
    let line = &lines(&[record(), record()])[0];
    assert!(!line.units_discarded.estimated);
    assert!(!line.weight_kg.estimated);
}

/// Annex I has one packaging column per line, so a line cannot hold weights that
/// differ on it.
#[test]
fn records_differing_on_packaging_do_not_share_a_line() {
    let packed = DiscardRecord {
        packaging_included: true,
        ..record()
    };
    let lines = lines(&[record(), packed]);

    assert_eq!(lines.len(), 2);
    assert_eq!(
        lines
            .iter()
            .map(|l| l.packaging_included)
            .collect::<Vec<_>>(),
        [false, true]
    );
}

/// Note (h): "a separate line is necessary for each reason".
#[test]
fn records_with_different_reasons_get_separate_lines() {
    let dangerous = DiscardRecord {
        reason: Some(DiscardReason::DangerousProduct),
        ..record()
    };
    let lines = lines(&[record(), dangerous]);

    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].reason, Some(DiscardReason::DangerousProduct));
    assert_eq!(lines[1].reason, Some(DiscardReason::DamagedOrContaminated));
}

/// A discard no derogation covers gets a line with no reason, whose detail is all
/// there is to say why; it sorts after the lines that cite one.
#[test]
fn a_discard_with_no_derogation_gets_a_line_with_no_reason() {
    let surplus = DiscardRecord {
        reason: None,
        reason_detail: Some("Surplus after a discontinued range".to_owned()),
        ..record()
    };
    let lines = lines(&[surplus, record()]);

    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].reason, Some(DiscardReason::DamagedOrContaminated));
    assert_eq!(lines[1].reason, None);
    assert_eq!(
        lines[1].reason_detail.as_deref(),
        Some("Surplus after a discontinued range")
    );
}

/// For a line with no reason the explanation *is* the reason, so two different
/// explanations are two reasons and two lines.
#[test]
fn records_with_different_explanations_do_not_share_a_line() {
    let explained = |text: &str| DiscardRecord {
        reason: None,
        reason_detail: Some(text.to_owned()),
        ..record()
    };
    assert_eq!(
        lines(&[explained("Overstock"), explained("Discontinued")]).len(),
        2
    );
    assert_eq!(
        lines(&[explained("Overstock"), explained("Overstock")]).len(),
        1
    );
}

/// Art. 3: two digits, except four for the products of Annex II.
#[test]
fn an_annex_ii_heading_files_at_four_digits_and_other_goods_at_two() {
    let fridge = DiscardRecord {
        cn_codes: vec![CommodityCode::parse("84181000").unwrap()],
        ..record()
    };
    let lines = lines(&[fridge, record()]);

    assert_eq!(lines[0].cn_categories, [category("62")]);
    assert_eq!(lines[1].cn_categories, [category("8418")]);
}

/// Different headings of one chapter are one category when the chapter is not
/// split by Annex II.
#[test]
fn headings_of_one_chapter_outside_annex_ii_share_a_line() {
    let blouse = DiscardRecord {
        cn_codes: vec![CommodityCode::parse("62064000").unwrap()],
        ..record()
    };
    let lines = lines(&[record(), blouse]);

    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].cn_categories, [category("62")]);
}

/// Note (f): "Multiple items sold together … may indicate more than one CN code."
#[test]
fn a_kit_is_filed_under_every_category_it_names() {
    let kit = DiscardRecord {
        cn_codes: vec![
            CommodityCode::parse("62034231").unwrap(),
            CommodityCode::parse("34011100").unwrap(),
            CommodityCode::parse("62064000").unwrap(),
        ],
        ..record()
    };
    let lines = lines(&[kit]);

    assert_eq!(lines[0].cn_categories, [category("3401"), category("62")]);
}

#[test]
fn a_kit_and_a_single_category_do_not_share_a_line() {
    let kit = DiscardRecord {
        cn_codes: vec![
            CommodityCode::parse("62034231").unwrap(),
            CommodityCode::parse("34011100").unwrap(),
        ],
        ..record()
    };
    assert_eq!(lines(&[kit, record()]).len(), 2);
}

/// Note (e): the description is "the combined nomenclature … or a more detailed
/// description".
#[test]
fn the_descriptions_of_a_line_are_joined_sorted_and_distinct() {
    let described = |text: &str| DiscardRecord {
        description: text.to_owned(),
        ..record()
    };
    let lines = lines(&[
        described("Trousers"),
        described("Blouses"),
        described("Trousers"),
    ]);

    assert_eq!(lines[0].description, "Blouses; Trousers");
}

/// Note (i): "calculated on the basis of the weight", so a heavy record outweighs
/// many light ones.
#[test]
fn treatment_shares_follow_weight_not_units() {
    let heavy = DiscardRecord {
        units: DiscardedQuantity::measured(1),
        weight: DiscardedWeight::measured(3_000),
        treatment: WasteTreatmentOperation::Recycling,
        ..record()
    };
    let many_light = DiscardRecord {
        units: DiscardedQuantity::measured(100),
        weight: DiscardedWeight::measured(1_000),
        treatment: WasteTreatmentOperation::Disposal,
        ..record()
    };
    let split = lines(&[heavy, many_light])[0].treatment;

    assert_eq!((split.recycling_pct, split.disposal_pct), (75, 25));
}

#[test]
fn treatment_shares_total_exactly_one_hundred() {
    let treated = |operation| DiscardRecord {
        weight: DiscardedWeight::measured(1_000),
        treatment: operation,
        ..record()
    };
    let split = lines(&[
        treated(WasteTreatmentOperation::Recycling),
        treated(WasteTreatmentOperation::OtherRecovery),
        treated(WasteTreatmentOperation::Disposal),
    ])[0]
        .treatment;

    assert_eq!(split.total_pct(), 100);
    assert_eq!(
        (
            split.recycling_pct,
            split.other_recovery_pct,
            split.disposal_pct
        ),
        (34, 33, 33)
    );
}

/// Note (i): an operation the operator could not establish is listed as `unknown`.
#[test]
fn an_unknown_treatment_is_its_own_share() {
    let unknown = DiscardRecord {
        treatment: WasteTreatmentOperation::Unknown,
        ..record()
    };
    let split = lines(&[unknown])[0].treatment;

    assert_eq!(split.unknown_pct, 100);
    assert_eq!(split.total_pct(), 100);
}

#[test]
fn preparing_for_reuse_is_its_own_share_and_is_not_destruction() {
    let reuse = DiscardRecord {
        treatment: WasteTreatmentOperation::PreparingForReuse,
        reason: None,
        ..record()
    };
    let split = lines(&[reuse])[0].treatment;

    assert_eq!(split.preparing_for_reuse_pct, 100);
    assert_eq!(split.total_destruction_pct(), 0);
}

/// Both ends of the financial year are in it.
#[test]
fn the_first_and_last_day_of_the_year_are_in_the_year() {
    let on = |d| DiscardRecord {
        discarded_on: d,
        ..record()
    };
    let aggregation = aggregate(&[on(date(2027, 1, 1)), on(date(2027, 12, 31))], &year());

    assert!(aggregation.excluded.is_empty());
    assert_eq!(aggregation.lines[0].units_discarded.value, 240);
}

/// A discard belongs to the disclosure of the year it was made in; one from another
/// year is reported, not summed into this one.
#[test]
fn a_record_outside_the_year_is_excluded_not_summed() {
    let early = DiscardRecord {
        discarded_on: date(2026, 12, 31),
        ..record()
    };
    let late = DiscardRecord {
        discarded_on: date(2028, 1, 1),
        ..record()
    };
    let aggregation = aggregate(&[early, record(), late], &year());

    assert_eq!(aggregation.lines[0].units_discarded.value, 120);
    assert_eq!(
        aggregation.excluded,
        [
            ExcludedRecord {
                index: 0,
                why: ExclusionReason::OutsideFinancialYear
            },
            ExcludedRecord {
                index: 2,
                why: ExclusionReason::OutsideFinancialYear
            },
        ]
    );
}

#[test]
fn a_record_with_no_cn_code_is_excluded() {
    let none = DiscardRecord {
        cn_codes: Vec::new(),
        ..record()
    };
    let aggregation = aggregate(&[none], &year());

    assert!(aggregation.lines.is_empty());
    assert_eq!(aggregation.excluded[0].why, ExclusionReason::NoCnCode);
}

#[test]
fn a_record_with_an_unreadable_cn_code_is_excluded() {
    for unreadable in ["6203", "abc", "620342310000"] {
        let record = DiscardRecord {
            cn_codes: vec![serde_json::from_value(serde_json::json!(unreadable)).unwrap()],
            ..record()
        };
        let aggregation = aggregate(&[record], &year());

        assert!(aggregation.lines.is_empty(), "{unreadable}");
        assert_eq!(
            aggregation.excluded[0].why,
            ExclusionReason::UnreadableCnCode,
            "{unreadable}"
        );
    }
}

/// Note (g) has the weight estimated from an accurate count, so a record carries
/// an estimate and not a zero; with none its share of the treatment cannot be worked
/// out.
#[test]
fn a_record_with_no_weight_is_excluded() {
    let weightless = DiscardRecord {
        weight: DiscardedWeight::measured(0),
        ..record()
    };
    let aggregation = aggregate(&[weightless], &year());

    assert!(aggregation.lines.is_empty());
    assert_eq!(aggregation.excluded[0].why, ExclusionReason::NoWeight);
}

#[test]
fn an_exclusion_names_the_record_by_its_position() {
    let none = DiscardRecord {
        cn_codes: Vec::new(),
        ..record()
    };
    let aggregation = aggregate(&[record(), record(), none], &year());

    assert_eq!(
        aggregation.excluded,
        [ExcludedRecord {
            index: 2,
            why: ExclusionReason::NoCnCode
        }]
    );
}

/// The same records in any order are the same disclosure.
#[test]
fn lines_come_out_in_a_fixed_order() {
    let fridge = DiscardRecord {
        cn_codes: vec![CommodityCode::parse("84181000").unwrap()],
        ..record()
    };
    let dangerous = DiscardRecord {
        reason: Some(DiscardReason::DangerousProduct),
        ..record()
    };
    let surplus = DiscardRecord {
        reason: None,
        ..record()
    };
    let forwards = [record(), fridge, dangerous, surplus];
    let mut backwards = forwards.clone();
    backwards.reverse();

    assert_eq!(lines(&forwards), lines(&backwards));
    assert_eq!(lines(&forwards).len(), 4);
}
