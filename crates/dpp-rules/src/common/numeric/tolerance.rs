//! Percentage and sum checks within a tolerance.

/// Whether `pct` is a valid percentage value: finite and in `[0.0, 100.0]`.
#[must_use]
pub fn percentage_in_range(pct: f64) -> bool {
    pct.is_finite() && (0.0..=100.0).contains(&pct)
}

/// Whether the values yielded by `values` sum to `target` within `±
/// tolerance` (percentage points). Returns the computed sum alongside the
/// verdict, since callers reporting a failure need it in their message. A
/// non-finite sum (e.g. one input was NaN) is always `false`.
#[must_use]
pub fn sums_to(values: impl IntoIterator<Item = f64>, target: f64, tolerance: f64) -> (bool, f64) {
    let total: f64 = values.into_iter().sum();
    let within_tolerance = total.is_finite() && (total - target).abs() <= tolerance;
    (within_tolerance, total)
}
