//! The record as data: what it reads and writes.

use serde_json::json;

use crate::test_support::sample_discard_record;
use crate::unsold_goods::DiscardRecord;

#[test]
fn a_record_round_trips_through_json() {
    let record = sample_discard_record();
    let wire = serde_json::to_value(&record).unwrap();
    let read: DiscardRecord = serde_json::from_value(wire).unwrap();
    assert_eq!(read, record);
}

/// No derogation is a real answer (Annex I note (h): "where applicable"), so it is
/// written as an absent key, never as `null` or a made-up word.
#[test]
fn a_record_with_no_derogation_writes_no_reason_key() {
    let record = DiscardRecord {
        reason: None,
        reason_detail: Some("Surplus after a discontinued range".to_owned()),
        ..sample_discard_record()
    };
    let wire = serde_json::to_value(&record).unwrap();

    assert!(wire.get("reason").is_none(), "{wire}");
    assert_eq!(
        wire["reasonDetail"],
        json!("Surplus after a discontinued range")
    );
    assert_eq!(
        serde_json::from_value::<DiscardRecord>(wire).unwrap(),
        record
    );
}

/// A malformed record is reported, not made unreadable: this one has no CN code at
/// all, and a store that could not read it back could not show it either.
#[test]
fn a_record_with_no_cn_code_stays_readable() {
    let record = DiscardRecord {
        cn_codes: Vec::new(),
        ..sample_discard_record()
    };
    let wire = serde_json::to_value(&record).unwrap();
    assert_eq!(
        serde_json::from_value::<DiscardRecord>(wire).unwrap(),
        record
    );
}

#[test]
fn a_record_with_a_code_that_is_not_a_cn_code_stays_readable() {
    let mut wire = serde_json::to_value(sample_discard_record()).unwrap();
    wire["cnCodes"] = json!(["not-a-code"]);
    let read: DiscardRecord = serde_json::from_value(wire).expect("a malformed code reads");
    assert_eq!(read.cn_codes[0].as_str(), "not-a-code");
}
