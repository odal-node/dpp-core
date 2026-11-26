//! What a [`FieldError`] looks like on a wire.

use super::super::FieldError;

/// What a caller puts on a wire: the two members, under these two names, and
/// nothing else. A rename here is a break for everything that reads it.
#[test]
fn a_field_error_serialises_as_field_and_message() {
    let error = FieldError {
        field: "/productGroupData/gtin".to_owned(),
        message: "the check digit is wrong".to_owned(),
    };

    let wire = serde_json::to_value(&error).expect("serialises");
    assert_eq!(
        wire,
        serde_json::json!({
            "field": "/productGroupData/gtin",
            "message": "the check digit is wrong"
        })
    );

    let back: FieldError = serde_json::from_value(wire).expect("reads back");
    assert_eq!(back, error);
}
