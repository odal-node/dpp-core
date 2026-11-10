//! Whole percentages that total exactly 100.

use proptest::prelude::*;

use crate::unsold_goods::apportion::whole_percentages;

/// Shares that already total 100 are returned as they are.
#[test]
fn exact_shares_need_no_correction() {
    assert_eq!(
        whole_percentages([0, 3_000, 0, 1_000, 0]),
        [0, 75, 0, 25, 0]
    );
}

/// Three equal thirds would each round to 33 and total 99. The point left over goes
/// to the lowest index among equal remainders.
#[test]
fn thirds_total_one_hundred_with_the_spare_point_to_the_first() {
    assert_eq!(whole_percentages([0, 1, 1, 1, 0]), [0, 34, 33, 33, 0]);
}

/// These shares are 33.5, 33.5 and 33.0. Rounding each to the nearest whole number
/// gives 34 + 34 + 33 = 101, which the disclosure lint would flag; flooring and
/// handing the one spare point to the first of the two equal remainders gives 100.
#[test]
fn shares_that_would_round_up_together_still_total_one_hundred() {
    let shares = whole_percentages([0, 67, 67, 66, 0]);
    assert_eq!(shares, [0, 34, 33, 33, 0]);
}

/// One operation holding all of the weight takes the whole 100.
#[test]
fn one_operation_takes_the_whole_line() {
    assert_eq!(whole_percentages([0, 0, 0, 0, 7]), [0, 0, 0, 0, 100]);
}

proptest! {
    /// Whatever the weights, the shares total 100 and none is more than one point
    /// from its exact value.
    #[test]
    fn shares_always_total_one_hundred_and_stay_within_a_point(
        weights in prop::array::uniform5(0u64..=u64::MAX),
    ) {
        prop_assume!(weights.iter().any(|w| *w > 0));
        let shares = whole_percentages(weights);

        prop_assert_eq!(shares.iter().map(|s| u32::from(*s)).sum::<u32>(), 100);

        let total: u128 = weights.iter().map(|w| u128::from(*w)).sum();
        for (weight, share) in weights.iter().zip(shares) {
            let exact_times_total = u128::from(*weight) * 100;
            let low = exact_times_total / total;
            prop_assert!(u128::from(share) == low || u128::from(share) == low + 1);
        }
    }
}
