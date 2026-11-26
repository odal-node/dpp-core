//! The shares of treatment a disclosure reports.

use crate::unsold_goods::DiscardedQuantity;
use crate::unsold_goods::{DiscardRecord, DiscardedWeight, WasteTreatmentOperation};

use super::common::*;

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

/// The five shares of a line always total exactly 100.
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

/// Preparing for reuse is a share of its own and does not count as destruction.
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
