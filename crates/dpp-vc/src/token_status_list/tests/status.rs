//! The Status Types of section 7.

use super::super::{Status, TokenStatusListError};

#[test]
fn the_three_registered_statuses_are_read() {
    assert_eq!(Status::from_value(0x00), Ok(Status::Valid));
    assert_eq!(Status::from_value(0x01), Ok(Status::Revoked));
    assert_eq!(Status::from_value(0x02), Ok(Status::Suspended));
}

#[test]
fn the_values_set_aside_for_applications_are_handed_back_as_they_are() {
    for value in [0x03, 0x0C, 0x0D, 0x0E, 0x0F] {
        assert_eq!(
            Status::from_value(value),
            Ok(Status::ApplicationSpecific(value))
        );
    }
}

/// Every other value is the draft's to register later. Reading one as valid, or as
/// anything else, would be guessing, so none is read.
#[test]
fn a_value_nobody_has_registered_is_refused_and_never_read_as_valid() {
    for value in (0x04..=0x0B).chain(0x10..=0xFF) {
        assert_eq!(
            Status::from_value(value),
            Err(TokenStatusListError::UnregisteredStatus(value)),
            "{value:#04x}"
        );
    }
}
