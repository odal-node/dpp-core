//! EU 2026/405 surfactant concentration band validation.

use alloc::{format, string::String};

/// EU-standard surfactant concentration bands (Annex VII of EU 2026/405).
pub const SURFACTANT_BANDS: &[&str] = &["<5%", "5-15%", "15-30%", ">=30%"];

/// A surfactant entry for validation.
#[derive(Debug, Clone, Copy)]
pub struct SurfactantInput<'a> {
    pub name: &'a str,
    pub concentration_band: &'a str,
    /// Must be `true`; non-biodegradable surfactants are prohibited (EU 2026/405 Art. 9).
    pub biodegradable: bool,
    pub cas_number: Option<&'a str>,
}

/// Whether a concentration band is one of the EU-standard [`SURFACTANT_BANDS`].
#[must_use]
pub fn surfactant_band_valid(band: &str) -> bool {
    SURFACTANT_BANDS.contains(&band)
}

/// Validate a detergent surfactant list: non-empty, each entry named, band valid,
/// and each surfactant marked biodegradable.
pub fn validate_surfactants(surfactants: &[SurfactantInput<'_>]) -> Result<(), String> {
    if surfactants.is_empty() {
        return Err(String::from("surfactants must not be empty"));
    }
    for (i, s) in surfactants.iter().enumerate() {
        if s.name.is_empty() {
            return Err(format!("surfactants[{i}]: name must not be empty"));
        }
        if !surfactant_band_valid(s.concentration_band) {
            return Err(format!(
                "surfactants[{i}]: concentration_band '{}' must be one of <5%, 5-15%, 15-30%, >=30%",
                s.concentration_band
            ));
        }
        if !s.biodegradable {
            return Err(format!(
                "surfactants[{i}]: '{}' must be readily biodegradable (EU 2026/405 Art. 9)",
                s.name
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn surf<'a>(name: &'a str, band: &'a str, biodegradable: bool) -> SurfactantInput<'a> {
        SurfactantInput {
            name,
            concentration_band: band,
            biodegradable,
            cas_number: None,
        }
    }

    #[test]
    fn surfactant_bands_and_validation() {
        assert!(surfactant_band_valid("5-15%"));
        assert!(!surfactant_band_valid("lots"));

        assert!(validate_surfactants(&[surf("LAS", "5-15%", true)]).is_ok());
        assert!(validate_surfactants(&[]).is_err());
        assert!(validate_surfactants(&[surf("LAS", "lots", true)]).is_err());
    }

    #[test]
    fn non_biodegradable_surfactant_rejected() {
        let err = validate_surfactants(&[surf("APEO", "5-15%", false)]).unwrap_err();
        assert!(err.contains("biodegradable"), "unexpected: {err}");
    }

    #[test]
    fn biodegradable_flag_checked_per_entry() {
        let ok = surf("LAS", "5-15%", true);
        let bad = surf("APEO", "<5%", false);
        let err = validate_surfactants(&[ok, bad]).unwrap_err();
        assert!(err.contains("surfactants[1]"), "unexpected: {err}");
    }

    #[test]
    fn cas_number_threaded_through() {
        let s = SurfactantInput {
            name: "LAS",
            concentration_band: "5-15%",
            biodegradable: true,
            cas_number: Some("25155-30-0"),
        };
        assert!(validate_surfactants(&[s]).is_ok());
    }
}
