//! Fibre composition sums and validation.

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
