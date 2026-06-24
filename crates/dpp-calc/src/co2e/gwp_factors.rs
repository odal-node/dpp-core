//! GWP100 characterisation factors from EF 3.1 (IPCC AR6).
//!
//! These are *multipliers* that convert kilograms of a substance into
//! kg CO₂-equivalents. They are **not** lifecycle-inventory (LCI) datasets —
//! the IPCC-derived coefficients are freely available and safe to embed in an
//! Apache-2.0 crate.
//!
//! Source: European Commission EF 3.1 method, which adopts IPCC AR6 GWP100
//! values (2021). Primary reference: IPCC AR6 WG-I Table 7.SM.7 (Supplementary
//! Material to Chapter 7, 2021). See also:
//! <https://eplca.jrc.ec.europa.eu/LCDN/developerEF.xhtml>

/// GWP100 factor for a substance by its common name/formula string.
///
/// Returns `None` for unknown substances; callers should fall back to a
/// `FactorProvider` for substances not listed here.
///
/// All values from IPCC AR6 WG-I Table 7.SM.7 (GWP100, no Earth-system feedbacks)
/// as adopted by the EC EF 3.1 characterisation method.
pub fn by_name(substance: &str) -> Option<f64> {
    Some(match substance {
        "CO2" | "carbon dioxide" => 1.0,
        // AR6 WG-I Table 7.SM.7 fossil CH4 GWP100 = 29.8.
        "CH4-fossil" | "methane fossil" => 29.8,
        // AR6 WG-I Table 7.SM.7 biogenic CH4 GWP100 = 27.9.
        // EF 3.1 uses 27.9 (not 27.0) because it attributes the CO₂ produced by
        // atmospheric CH4 oxidation back to the biogenic CH4 source, consistent
        // with the EF 3.1 system-boundary treatment of biogenic carbon flows.
        "CH4-biogenic" | "methane biogenic" => 27.9,
        // AR6 WG-I Table 7.SM.7: N₂O GWP100 = 273.
        "N2O" | "nitrous oxide" => 273.0,
        // AR6 WG-I Table 7.SM.7: SF₆ GWP100 = 25_200 (no Earth-system feedbacks).
        // Previous code used 23_500 (IPCC AR5); AR6 raised this by ~7 %.
        // Source confirmed: IPCC AR6 WG-I Chapter 7 Supplementary Material Table 7.SM.7.
        "SF6" | "sulphur hexafluoride" => 25_200.0,
        // AR6 WG-I Table 7.SM.7: NF₃ GWP100 = 17_400 (unchanged from AR5).
        "NF3" | "nitrogen trifluoride" => 17_400.0,
        "HFC-134a" | "1,1,1,2-tetrafluoroethane" => 1_526.0,
        "HFC-32" | "difluoromethane" => 771.0,
        "HFC-125" | "pentafluoroethane" => 3_740.0,
        "HFC-143a" | "1,1,1-trifluoroethane" => 5_810.0,
        "HFC-152a" | "1,1-difluoroethane" => 164.0,
        "HFC-227ea" | "1,1,1,2,3,3,3-heptafluoropropane" => 3_600.0,
        "HFC-245fa" | "1,1,1,3,3-pentafluoropropane" => 858.0,
        "PFC-14" | "CF4" | "tetrafluoromethane" => 7_380.0,
        "PFC-116" | "C2F6" | "hexafluoroethane" => 12_400.0,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn co2_factor_is_one() {
        assert_eq!(by_name("CO2"), Some(1.0));
    }

    #[test]
    fn methane_fossil_ar6() {
        assert_eq!(by_name("CH4-fossil"), Some(29.8));
    }

    /// CH4-biogenic GWP100 = 27.9 per EF 3.1 / AR6 WG-I Table 7.SM.7.
    /// EF 3.1 uses 27.9 (not 27.0) because it attributes the CO₂ from CH4
    /// oxidation back to the biogenic emission source. Changing this value
    /// requires a cited update to the EF 3.1 characterisation table.
    #[test]
    fn methane_biogenic_ef31() {
        assert_eq!(by_name("CH4-biogenic"), Some(27.9));
        assert_eq!(by_name("methane biogenic"), Some(27.9));
    }

    #[test]
    fn n2o_ar6() {
        assert_eq!(by_name("N2O"), Some(273.0));
    }

    /// SF₆ GWP100 = 25_200 per IPCC AR6 WG-I Table 7.SM.7 (no Earth-system
    /// feedbacks), as adopted by EF 3.1. The previous AR5 value was 23_500 —
    /// a ~7 % underestimate that materially affects electronics and semiconductor
    /// passports. Any change here must cite the updated IPCC/EF source table.
    #[test]
    fn sf6_is_ar6_value_not_ar5() {
        let gwp = by_name("SF6").expect("SF6 must be in the table");
        assert!(
            (gwp - 25_200.0).abs() < 1.0,
            "SF6 GWP100 must be 25_200 (AR6 WG-I Table 7.SM.7), got {gwp}"
        );
        // Explicitly reject the old AR5 value so a silent revert is caught.
        assert!(
            (gwp - 23_500.0).abs() > 1.0,
            "SF6 must not be the AR5 value 23_500"
        );
        // Both aliases resolve to the same value.
        assert_eq!(by_name("sulphur hexafluoride"), Some(25_200.0));
    }

    #[test]
    fn hfc_and_pfc_spot_check() {
        assert_eq!(by_name("HFC-134a"), Some(1_526.0));
        assert_eq!(by_name("HFC-32"), Some(771.0));
        assert_eq!(by_name("PFC-14"), Some(7_380.0));
        assert_eq!(by_name("CF4"), Some(7_380.0));
    }

    #[test]
    fn unknown_returns_none() {
        assert_eq!(by_name("unobtanium"), None);
    }
}
