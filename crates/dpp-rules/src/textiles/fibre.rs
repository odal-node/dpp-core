//! Fibre composition validation — EU textile DPP regulation.

use alloc::{format, string::String};

use crate::common::country::country_code_valid;
use crate::common::numeric::{percentage_in_range, sums_to};

// ⚠️ COMPLIANCE-PIN PENDING (watchlist 🟠): 2.0 pp tolerance is the commonly
// cited value but confirm against EU 1007/2011 Annex IX (agreed tolerance for
// fibre composition declarations) before PINning.
/// Tolerance (percentage points) allowed when fibre percentages are summed.
pub const FIBRE_SUM_TOLERANCE: f64 = 2.0;

/// A fibre entry for composition validation.
#[derive(Debug, Clone, Copy)]
pub struct FibreInput<'a> {
    pub fibre: &'a str,
    pub pct: f64,
    pub country_of_origin: Option<&'a str>,
}

/// Whether fibre percentages sum to ~100% (± [`FIBRE_SUM_TOLERANCE`]).
///
/// Used by plugins for the compliance determination and by
/// [`validate_fibre_composition`] for the cross-field validation rule.
#[must_use]
pub fn fibre_sum_ok(pcts: &[f64]) -> bool {
    sums_to(pcts.iter().copied(), 100.0, FIBRE_SUM_TOLERANCE).0
}

/// Validate a textile fibre composition: non-empty, each `pct` in `[0, 100]`,
/// any `country_of_origin` a valid ISO 3166-1 alpha-2 code, and the percentages
/// summing to ~100% (± [`FIBRE_SUM_TOLERANCE`]).
pub fn validate_fibre_composition(fibres: &[FibreInput<'_>]) -> Result<(), String> {
    if fibres.is_empty() {
        return Err(String::from("fibre_composition must not be empty"));
    }
    for f in fibres {
        if !percentage_in_range(f.pct) {
            return Err(format!(
                "fibre '{}' has invalid pct {} — must be a finite value in 0–100",
                f.fibre, f.pct
            ));
        }
        if let Some(co) = f.country_of_origin
            && !country_code_valid(co)
        {
            return Err(format!(
                "fibre '{}' has invalid country_of_origin '{}' — must be ISO 3166-1 alpha-2",
                f.fibre, co
            ));
        }
    }
    let (sum_ok, total) = sums_to(fibres.iter().map(|f| f.pct), 100.0, FIBRE_SUM_TOLERANCE);
    if !sum_ok {
        return Err(format!(
            "fibreComposition percentages sum to {total:.1}, expected 100.0 (± {FIBRE_SUM_TOLERANCE:.1})"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fibre<'a>(name: &'a str, pct: f64) -> FibreInput<'a> {
        FibreInput {
            fibre: name,
            pct,
            country_of_origin: None,
        }
    }

    #[test]
    fn fibre_sum_within_tolerance() {
        assert!(fibre_sum_ok(&[60.0, 40.0]));
        assert!(fibre_sum_ok(&[98.5, 1.0])); // 99.5 within ±2
        assert!(!fibre_sum_ok(&[60.0, 30.0])); // 90
        assert!(!fibre_sum_ok(&[])); // empty → 0
    }

    #[test]
    fn fibre_composition_valid_passes() {
        assert!(
            validate_fibre_composition(&[fibre("cotton", 60.0), fibre("polyester", 40.0)]).is_ok()
        );
    }

    #[test]
    fn fibre_sum_invalid_message_has_total() {
        let err = validate_fibre_composition(&[fibre("cotton", 60.0), fibre("polyester", 30.0)])
            .unwrap_err();
        assert!(err.contains("90.0"), "unexpected: {err}");
    }

    #[test]
    fn fibre_invalid_country_rejected() {
        let entry = FibreInput {
            fibre: "cotton",
            pct: 100.0,
            country_of_origin: Some("india"),
        };
        let err = validate_fibre_composition(&[entry]).unwrap_err();
        assert!(err.contains("country_of_origin"), "unexpected: {err}");
    }

    #[test]
    fn fibre_empty_rejected() {
        assert!(validate_fibre_composition(&[]).is_err());
    }

    #[test]
    fn nan_pct_rejected() {
        let err = validate_fibre_composition(&[fibre("cotton", f64::NAN)]).unwrap_err();
        assert!(
            err.contains("NaN") || err.contains("finite"),
            "unexpected: {err}"
        );
    }

    #[test]
    fn infinity_pct_rejected() {
        let err = validate_fibre_composition(&[fibre("cotton", f64::INFINITY)]).unwrap_err();
        assert!(
            err.contains("inf") || err.contains("finite"),
            "unexpected: {err}"
        );
    }
}
