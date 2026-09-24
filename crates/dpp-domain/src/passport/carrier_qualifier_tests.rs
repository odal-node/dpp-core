//! The carrier qualifier: which level a passport's carrier asserts, and what
//! `validate` requires of the identifiers at that level.

use std::borrow::Cow;

use super::tests::make_passport;
use super::{CarrierQualifier, Passport};
use crate::Granularity;
use crate::error::dpp::DppError;

fn at(level: Option<Granularity>) -> Passport {
    let mut p = make_passport();
    p.granularity = level;
    p.batch_id = None;
    p.serial_number = None;
    p
}

fn fields_refused(p: &Passport) -> Vec<(String, String)> {
    match p.validate() {
        Ok(()) => Vec::new(),
        Err(DppError::Validation(errors)) => errors
            .errors
            .into_iter()
            .map(|e| (e.field, e.message))
            .collect(),
        Err(other) => panic!("expected a validation error, got {other:?}"),
    }
}

/// The defect: a model-level passport's carrier printed a serial, which with a
/// GTIN names one individual item, on every unit of the model.
#[test]
fn a_model_level_carrier_prints_no_serial() {
    let p = at(Some(Granularity::Model));
    assert_eq!(p.carrier_qualifier(), Some(CarrierQualifier::Model));
    assert_eq!(CarrierQualifier::Model.ai_element(), None);
}

/// A batch-level carrier prints its lot in AI 10, and no serial.
#[test]
fn a_batch_level_carrier_prints_its_lot() {
    let mut p = at(Some(Granularity::Batch));
    p.batch_id = Some("LOT-2026-A".into());
    let qualifier = p.carrier_qualifier().expect("a batch to print");
    assert_eq!(
        qualifier,
        CarrierQualifier::Batch(Cow::Borrowed("LOT-2026-A"))
    );
    assert_eq!(qualifier.ai_element(), Some(("10", "LOT-2026-A")));
}

/// Item level, and a level not stated, print the carrier serial — and only
/// the carrier serial, whatever lot or manufacturer's serial the record holds.
#[test]
fn an_item_or_unstated_carrier_prints_the_carrier_serial() {
    for level in [Some(Granularity::Item), None] {
        let mut p = at(level);
        p.batch_id = Some("LOT-2026-A".into());
        p.serial_number = Some("SN-0001".into());
        let qualifier = p.carrier_qualifier().expect("a serial to print");
        assert_eq!(
            qualifier.ai_element(),
            Some(("21", &*p.effective_carrier_serial())),
            "{level:?}"
        );
    }
}

/// A batch-level passport with no lot has nothing to print, and says so
/// rather than falling back to a serial that would claim an individual item.
#[test]
fn a_batch_level_passport_without_a_lot_has_no_carrier() {
    assert_eq!(at(Some(Granularity::Batch)).carrier_qualifier(), None);
}

/// A model-level record naming a batch or a unit describes that batch or unit,
/// not the model.
#[test]
fn validate_refuses_a_model_level_passport_naming_a_batch_or_unit() {
    let mut p = at(Some(Granularity::Model));
    assert!(p.validate().is_ok());

    p.batch_id = Some("LOT-A".into());
    p.serial_number = Some("SN-0001".into());
    let refused: Vec<_> = fields_refused(&p).into_iter().map(|(f, _)| f).collect();
    assert_eq!(refused, ["/batchId", "/serialNumber"]);
}

/// A batch-level record needs the lot its carrier prints, held to GS1 AI 10,
/// and names no unit.
#[test]
fn validate_requires_a_printable_lot_at_batch_level() {
    let mut p = at(Some(Granularity::Batch));
    let refused = fields_refused(&p);
    assert_eq!(refused.len(), 1);
    assert_eq!(refused[0].0, "/batchId");
    assert!(refused[0].1.contains("GS1 AI 10"), "{}", refused[0].1);

    p.batch_id = Some("LOT-2026-A".into());
    assert!(p.validate().is_ok());

    for (bad, expected) in [
        ("", "must not be empty"),
        ("123456789012345678901", "GS1 AI 10 allows at most 20"),
        ("LOT A", "outside GS1 CSET 82"),
        ("LOT#A", "outside GS1 CSET 82"),
    ] {
        p.batch_id = Some(bad.into());
        let refused = fields_refused(&p);
        assert_eq!(refused.len(), 1, "{bad:?}");
        assert_eq!(refused[0].0, "/batchId", "{bad:?}");
        assert!(refused[0].1.contains(expected), "{bad:?}: {}", refused[0].1);
    }

    p.batch_id = Some("LOT-2026-A".into());
    p.serial_number = Some("SN-0001".into());
    let refused: Vec<_> = fields_refused(&p).into_iter().map(|(f, _)| f).collect();
    assert_eq!(refused, ["/serialNumber"]);
}

/// At item level, or with no level stated, the lot is free text: nothing
/// prints it, so nothing holds it to GS1.
#[test]
fn validate_leaves_the_lot_free_where_no_carrier_prints_it() {
    for level in [Some(Granularity::Item), None] {
        let mut p = at(level);
        p.batch_id = Some("Lot A, line 3 #7".into());
        p.serial_number = Some("SN-0001".into());
        assert!(p.validate().is_ok(), "{level:?}");
    }
}
