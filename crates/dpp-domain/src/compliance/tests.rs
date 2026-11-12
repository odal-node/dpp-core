//! Determination gating, and how findings split into violations and warnings.

use super::*;

#[test]
fn provisional_downgrades_binding_determinations() {
    assert_eq!(
        gate_determination(false, ComplianceStatus::Compliant),
        ComplianceStatus::NotAssessed
    );
    assert_eq!(
        gate_determination(false, ComplianceStatus::NonCompliant),
        ComplianceStatus::NotAssessed
    );
}

#[test]
fn in_force_preserves_determinations() {
    assert_eq!(
        gate_determination(true, ComplianceStatus::Compliant),
        ComplianceStatus::Compliant
    );
    assert_eq!(
        gate_determination(true, ComplianceStatus::NonCompliant),
        ComplianceStatus::NonCompliant
    );
}

#[test]
fn a_bound_determination_writes_its_input_hash() {
    let hash = "ab".repeat(32);
    let bound = ComplianceResult {
        input_hash: Some(hash.clone()),
        ..ComplianceResult::default()
    };

    let wire = serde_json::to_value(&bound).unwrap();
    assert_eq!(wire["inputHash"].as_str(), Some(hash.as_str()));
}

#[test]
fn a_determination_without_an_input_hash_writes_and_reads_as_before() {
    let wire = serde_json::to_value(ComplianceResult::default()).unwrap();
    assert!(wire.get("inputHash").is_none());

    let read: ComplianceResult = serde_json::from_value(wire).unwrap();
    assert_eq!(read.input_hash, None);
}

#[test]
fn non_binding_statuses_pass_through_regardless() {
    for in_force in [true, false] {
        assert_eq!(
            gate_determination(in_force, ComplianceStatus::PassthroughNoValidation),
            ComplianceStatus::PassthroughNoValidation
        );
        assert_eq!(
            gate_determination(in_force, ComplianceStatus::NotAssessed),
            ComplianceStatus::NotAssessed
        );
    }
}
