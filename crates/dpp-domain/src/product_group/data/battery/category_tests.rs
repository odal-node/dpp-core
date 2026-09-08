//! Battery category wire tags.

use super::*;

/// `wire_str()` is what `product_category()` hands to a scope check, so a
/// disagreement with serde would scope a credential against a value no stored
/// passport contains. `Sli` is the variant that makes this real: it is the one
/// whose tag is not its identifier lowercased.
#[test]
fn battery_type_wire_str_matches_serde() {
    for category in [
        BatteryType::Portable,
        BatteryType::Industrial,
        BatteryType::Ev,
        BatteryType::Lmt,
        BatteryType::Sli,
    ] {
        let serialized = serde_json::to_value(&category).unwrap();
        assert_eq!(
            serialized.as_str().unwrap(),
            category.wire_str(),
            "wire_str() disagrees with serde for {category:?}"
        );
    }
}
