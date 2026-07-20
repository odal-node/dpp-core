//! Shared numeric helpers: percentage clamping, sum checks, thresholds.

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentage_in_range_accepts_boundaries() {
        assert!(percentage_in_range(0.0));
        assert!(percentage_in_range(100.0));
        assert!(percentage_in_range(50.0));
    }

    #[test]
    fn percentage_in_range_rejects_out_of_bounds_and_non_finite() {
        assert!(!percentage_in_range(-0.001));
        assert!(!percentage_in_range(100.001));
        assert!(!percentage_in_range(f64::NAN));
        assert!(!percentage_in_range(f64::INFINITY));
    }

    #[test]
    fn sums_to_within_tolerance() {
        let (ok, total) = sums_to([60.0, 40.0], 100.0, 2.0);
        assert!(ok);
        assert!((total - 100.0).abs() < f64::EPSILON);

        let (ok, total) = sums_to([98.5, 1.0], 100.0, 2.0);
        assert!(ok, "99.5 is within ±2 of 100");
        assert!((total - 99.5).abs() < f64::EPSILON);
    }

    #[test]
    fn sums_to_outside_tolerance() {
        let (ok, total) = sums_to([60.0, 30.0], 100.0, 2.0);
        assert!(!ok);
        assert!((total - 90.0).abs() < f64::EPSILON);
    }

    #[test]
    fn sums_to_non_finite_input_is_never_ok() {
        let (ok, total) = sums_to([f64::NAN, 50.0], 100.0, 2.0);
        assert!(!ok);
        assert!(total.is_nan());
    }

    #[test]
    fn sums_to_empty_is_zero() {
        let (ok, total) = sums_to([], 100.0, 2.0);
        assert!(!ok);
        assert_eq!(total, 0.0);
    }
}
