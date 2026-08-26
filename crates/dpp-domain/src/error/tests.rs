//! How each domain error renders.

use super::dpp::*;

#[test]
fn not_found_display() {
    let e = DppError::NotFound("passport-123".to_owned());
    assert_eq!(e.to_string(), "passport not found: passport-123");
}

#[test]
fn invalid_transition_display() {
    let e = DppError::InvalidTransition {
        current: "archived".to_owned(),
        required: "draft".to_owned(),
    };
    let msg = e.to_string();
    assert!(
        msg.contains("archived"),
        "message should contain current state"
    );
    assert!(
        msg.contains("draft"),
        "message should contain required state"
    );
}

#[test]
fn validation_display() {
    let e = DppError::Validation("product_name is required".into());
    assert!(e.to_string().contains("product_name is required"));
}
