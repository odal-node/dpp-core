//! The Annex I checks of [`calculate`](super::calculate), one case each: a share
//! that is missing, fractional or out of range is an error, never a zero.

use super::*;
use serde_json::{Value, json};

fn line_with_shares(shares: Value) -> Value {
    json!({ "treatment": shares })
}

#[test]
fn shares_summing_to_one_hundred_total_one_hundred() {
    let line = line_with_shares(json!({
        "preparingForReusePct": 40, "recyclingPct": 30, "otherRecoveryPct": 10,
        "disposalPct": 15, "unknownPct": 5,
    }));
    assert_eq!(treatment_total(&line), Ok(100));
}

#[test]
fn a_share_too_large_for_u32_is_refused_rather_than_truncated() {
    // 2^32 + 100 truncates to 100 under an unchecked `as u32`, which would make
    // the other four zero shares look like a complete split.
    let line = line_with_shares(json!({
        "preparingForReusePct": 4_294_967_396_u64, "recyclingPct": 0,
        "otherRecoveryPct": 0, "disposalPct": 0, "unknownPct": 0,
    }));
    assert!(treatment_total(&line).is_err());
}

#[test]
fn a_missing_or_fractional_share_is_refused_rather_than_read_as_zero() {
    let missing = line_with_shares(json!({
        "preparingForReusePct": 100, "recyclingPct": 0,
        "otherRecoveryPct": 0, "disposalPct": 0,
    }));
    assert!(treatment_total(&missing).is_err());

    let fractional = line_with_shares(json!({
        "preparingForReusePct": 99.5, "recyclingPct": 0,
        "otherRecoveryPct": 0, "disposalPct": 0, "unknownPct": 0,
    }));
    assert!(treatment_total(&fractional).is_err());
}
