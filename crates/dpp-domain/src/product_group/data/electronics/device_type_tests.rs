//! Device type wire tags.

use super::*;

/// `wire_str()` is what `product_category()` hands to a scope check, so a
/// disagreement with serde would scope a credential against a value no stored
/// passport contains. `Tablet` is the variant that makes this real: every other
/// tag follows the `kebab-case` rule and it does not.
#[test]
fn device_type_wire_str_matches_serde() {
    for device in [
        DeviceType::Smartphone,
        DeviceType::OtherMobilePhone,
        DeviceType::CordlessPhone,
        DeviceType::Tablet,
    ] {
        let serialized = serde_json::to_value(&device).unwrap();
        assert_eq!(
            serialized.as_str().unwrap(),
            device.wire_str(),
            "wire_str() disagrees with serde for {device:?}"
        );
    }
}
