//! The bound on a carbon footprint class label, and what it refuses.

use super::*;

#[test]
fn round_trip_preserves_a_label_this_build_has_never_seen() {
    // The regression this type exists to prevent. The previous enum mapped
    // every unrecognised label to `Other`, so a future scale's "F" and "A+"
    // both round-tripped to the string "Other" — under a qualified seal.
    for label in ["A", "F", "A+", "A+++", "CLASS-1"] {
        let json = serde_json::to_string(&CarbonFootprintClass::new(label).unwrap()).unwrap();
        assert_eq!(json, format!("\"{label}\""));
        let back: CarbonFootprintClass = serde_json::from_str(&json).unwrap();
        assert_eq!(back.as_str(), label, "label must survive the round trip");
    }
}

#[test]
fn deserialization_validates_rather_than_silently_accepting() {
    // A catch-all variant would have swallowed these. `try_from` rejects
    // them at the boundary, where the caller can still report a field path.
    for bad in ["\"\"", "\"   \"", "\"TOOLONGLABEL\""] {
        assert!(
            serde_json::from_str::<CarbonFootprintClass>(bad).is_err(),
            "should reject {bad}"
        );
    }
}

#[test]
fn rejects_empty_blank_overlong_and_control_characters() {
    assert!(matches!(
        CarbonFootprintClass::new(""),
        Err(CarbonFootprintClassError::Empty)
    ));
    assert!(matches!(
        CarbonFootprintClass::new("  \t "),
        Err(CarbonFootprintClassError::Empty)
    ));
    assert!(matches!(
        CarbonFootprintClass::new("A\u{7}"),
        Err(CarbonFootprintClassError::ControlCharacter(_))
    ));
    let err = CarbonFootprintClass::new("ABCDEFGHI").unwrap_err();
    assert!(matches!(
        err,
        CarbonFootprintClassError::TooLong { len: 9, max: 8, .. }
    ));
}

#[test]
fn length_bound_matches_the_schema_and_counts_characters() {
    assert_eq!(CarbonFootprintClass::MAX_LEN, 8);
    // Exactly at the bound is accepted.
    assert!(CarbonFootprintClass::new("ABCDEFGH").is_ok());
    // maxLength in JSON Schema counts code points, not bytes — a
    // byte-length check would wrongly reject this 4-character label.
    assert!(CarbonFootprintClass::new("Ä+++").is_ok());
}

#[test]
fn label_is_stored_verbatim_without_case_folding() {
    let c = CarbonFootprintClass::new("a+").unwrap();
    assert_eq!(c.as_str(), "a+");
    assert_eq!(c.to_string(), "a+");
}
