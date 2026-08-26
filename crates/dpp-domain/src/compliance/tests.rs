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
