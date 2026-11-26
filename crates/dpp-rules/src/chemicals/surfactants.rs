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
mod tests;
