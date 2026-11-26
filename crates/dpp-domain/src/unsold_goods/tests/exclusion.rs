//! The records a year excludes, and the order its lines come out in.

use crate::identifier::DeclaredCommodityCode;
use crate::unsold_goods::DiscardReason;
use crate::unsold_goods::{
    DiscardRecord, DiscardedWeight, ExcludedRecord, ExclusionReason, LineDescriptions, aggregate,
};

use super::common::*;

/// Both ends of the financial year are in it.
#[test]
fn the_first_and_last_day_of_the_year_are_in_the_year() {
    let on = |d| DiscardRecord {
        discarded_on: d,
        ..record()
    };
    let aggregation = aggregate(
        &[on(date(2027, 1, 1)), on(date(2027, 12, 31))],
        &year(),
        &LineDescriptions::new(),
    );

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
    let aggregation = aggregate(&[early, record(), late], &year(), &LineDescriptions::new());

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

/// A record naming no CN code cannot be filed, so it is returned as excluded.
#[test]
fn a_record_with_no_cn_code_is_excluded() {
    let none = DiscardRecord {
        cn_codes: Vec::new(),
        ..record()
    };
    let aggregation = aggregate(&[none], &year(), &LineDescriptions::new());

    assert!(aggregation.lines.is_empty());
    assert_eq!(aggregation.excluded[0].why, ExclusionReason::NoCnCode);
}

/// A record whose CN code is unreadable is returned as excluded.
#[test]
fn a_record_with_an_unreadable_cn_code_is_excluded() {
    for unreadable in ["6203", "abc", "620342310000"] {
        let record = DiscardRecord {
            cn_codes: vec![serde_json::from_value(serde_json::json!(unreadable)).unwrap()],
            ..record()
        };
        let aggregation = aggregate(&[record], &year(), &LineDescriptions::new());

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
    let aggregation = aggregate(&[weightless], &year(), &LineDescriptions::new());

    assert!(aggregation.lines.is_empty());
    assert_eq!(aggregation.excluded[0].why, ExclusionReason::NoWeight);
}

/// An exclusion names the record by its position in the slice aggregated.
#[test]
fn an_exclusion_names_the_record_by_its_position() {
    let none = DiscardRecord {
        cn_codes: Vec::new(),
        ..record()
    };
    let aggregation = aggregate(
        &[record(), record(), none],
        &year(),
        &LineDescriptions::new(),
    );

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
        cn_codes: vec![DeclaredCommodityCode::new("84181000")],
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
