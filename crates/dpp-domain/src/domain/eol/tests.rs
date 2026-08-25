//! End-of-life events: what they require, and what they refuse.

use super::*;
use crate::domain::passport::PassportId;

#[test]
fn recycled_roundtrips_and_needs_no_derogation() {
    let e = EolEvent::new(PassportId::new(), DeactivationReason::Recycled, "did:web:r");
    assert!(!e.requires_derogation());
    let v = serde_json::to_value(&e).unwrap();
    assert_eq!(v["reason"]["kind"], "recycled");
    let back: EolEvent = serde_json::from_value(v).unwrap();
    assert_eq!(back, e);
}

#[test]
fn destroyed_carries_a_derogation() {
    let e = EolEvent::new(
        PassportId::new(),
        DeactivationReason::Destroyed {
            derogation: DerogationRef {
                category: "health-and-safety".into(),
                act_citation: Some("Delegated Reg. (EU) 2026/xxx".into()),
            },
        },
        "did:web:r",
    );
    assert!(e.requires_derogation());
    let v = serde_json::to_value(&e).unwrap();
    assert_eq!(v["reason"]["kind"], "destroyed");
    assert_eq!(v["reason"]["derogation"]["category"], "health-and-safety");
}
