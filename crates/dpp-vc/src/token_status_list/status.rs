//! The Status Types of draft-ietf-oauth-status-list-21 section 7.

use super::error::TokenStatusListError;

/// What a list says about a Referenced Token.
///
/// Not `#[non_exhaustive]`, on purpose: a caller's `match` should stop compiling
/// when a status is added, and not fall into a wildcard arm that reads it as valid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// `0x00` VALID.
    Valid,
    /// `0x01`, which the draft names INVALID: revoked, annulled, taken back,
    /// recalled or cancelled. Named for what it means to a verifier.
    Revoked,
    /// `0x02` SUSPENDED: invalid for now, and it can be undone.
    Suspended,
    /// A value the draft sets aside for applications, `0x03` and `0x0C` to
    /// `0x0F`, and which means whatever the application that wrote the list says
    /// it does. A caller that does not know that meaning must not read it as valid.
    ApplicationSpecific(u8),
}

impl Status {
    /// Read one entry of a list.
    ///
    /// # Errors
    ///
    /// [`TokenStatusListError::UnregisteredStatus`] for a value the draft keeps
    /// for future registration. Such a value is refused rather than mapped to the
    /// nearest status, because the one it would be mapped to is a guess.
    pub fn from_value(value: u8) -> Result<Self, TokenStatusListError> {
        match value {
            0x00 => Ok(Self::Valid),
            0x01 => Ok(Self::Revoked),
            0x02 => Ok(Self::Suspended),
            0x03 | 0x0C..=0x0F => Ok(Self::ApplicationSpecific(value)),
            _ => Err(TokenStatusListError::UnregisteredStatus(value)),
        }
    }
}
