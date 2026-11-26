//! Global warming potential lookup by substance name.

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
