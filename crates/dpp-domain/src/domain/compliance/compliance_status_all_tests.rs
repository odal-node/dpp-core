//! Every [`ComplianceStatus`] variant is reachable and round-trips.

use super::ComplianceStatus;

/// `ALL` must list every variant — see `PassportStatus`'s equivalent test.
#[test]
fn all_lists_every_variant() {
    for status in ComplianceStatus::ALL {
        match status {
            ComplianceStatus::PassthroughNoValidation
            | ComplianceStatus::Compliant
            | ComplianceStatus::NonCompliant
            | ComplianceStatus::NotAssessed
            | ComplianceStatus::NotImplemented => {}
        }
    }
    assert_eq!(
        ComplianceStatus::ALL.len(),
        5,
        "a variant was added to the match above but not to ALL"
    );
}
