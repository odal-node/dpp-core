//! Whole percentages that total exactly 100.

/// The share of the total each weight is, as whole percentages that sum to 100.
///
/// Rounding each share to the nearest whole number can total 99 or 101, and a
/// treatment split that does not total 100 is one the disclosure lint flags.
/// So shares are floored and the points left over go, one each, to the shares
/// with the largest remainders (the lower index first on a tie). No share ends up
/// more than one point from its exact value.
///
/// At least one weight must be above zero; with none there is nothing to apportion
/// and every share is 0.
pub(crate) fn whole_percentages(weights: [u64; 5]) -> [u8; 5] {
    let total: u128 = weights.iter().map(|w| u128::from(*w)).sum();
    if total == 0 {
        return [0; 5];
    }

    let mut shares = [0u8; 5];
    let mut remainders = [0u128; 5];
    for (i, weight) in weights.iter().enumerate() {
        let scaled = u128::from(*weight) * 100;
        // At most 100, because `weight <= total`.
        shares[i] = u8::try_from(scaled / total).unwrap_or(100);
        remainders[i] = scaled % total;
    }

    let leftover = 100 - shares.iter().map(|s| usize::from(*s)).sum::<usize>();
    let mut by_remainder = [0, 1, 2, 3, 4];
    by_remainder.sort_by(|a, b| remainders[*b].cmp(&remainders[*a]).then(a.cmp(b)));
    for index in by_remainder.into_iter().take(leftover) {
        shares[index] += 1;
    }
    shares
}
