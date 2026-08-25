//! Chemistry parsing and round-trips.

use super::*;

#[test]
fn battery_chemistry_wire_str_matches_serde_serialization() {
    for chem in [
        BatteryChemistry::Lfp,
        BatteryChemistry::Nmc,
        BatteryChemistry::Nca,
        BatteryChemistry::Lco,
        BatteryChemistry::NiMh,
        BatteryChemistry::NiCd,
        BatteryChemistry::LeadAcid,
        BatteryChemistry::SolidState,
        BatteryChemistry::Other,
    ] {
        let serialized = serde_json::to_value(&chem).unwrap();
        assert_eq!(
            serialized.as_str().unwrap(),
            chem.wire_str(),
            "wire_str() disagrees with serde for {chem:?}"
        );
    }
}
