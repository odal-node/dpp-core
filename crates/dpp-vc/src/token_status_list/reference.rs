//! The reference a credential carries to its entry in a list.

use serde_json::{Map, Value, json};

use super::error::TokenStatusListError;

/// Where a credential's status is kept: an index into the list at a URI.
///
/// Carried in the credential's `status` claim (section 6.2), which for an SD-JWT VC
/// sits at the root and is never concealed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusReference {
    /// `idx`, the entry of the list that holds this credential's status.
    pub idx: u64,
    /// `uri`, the address of the Status List Token that holds the list. The
    /// token's `sub` must equal it.
    pub uri: String,
}

impl StatusReference {
    /// The value of the `status` claim: `{"status_list": {"idx": …, "uri": …}}`.
    #[must_use]
    pub fn to_claim(&self) -> Value {
        json!({ "status_list": { "idx": self.idx, "uri": self.uri } })
    }

    /// Read the reference from a credential's claims.
    ///
    /// `Ok(None)` when there is no `status` claim, or when it names another status
    /// mechanism and no `status_list`: the draft lets a credential carry several,
    /// and this crate reads one.
    ///
    /// # Errors
    ///
    /// [`TokenStatusListError::MalformedReference`] when `status` is present and is
    /// not an object, or `status_list` is present and is not an object holding a
    /// non-negative integer `idx` and a non-empty string `uri`. A reference that
    /// cannot be read is refused and not taken for an absent one, since an absent
    /// one is a credential nobody can withdraw.
    pub fn from_claims(claims: &Map<String, Value>) -> Result<Option<Self>, TokenStatusListError> {
        let Some(status) = claims.get("status") else {
            return Ok(None);
        };
        let Some(status) = status.as_object() else {
            return Err(TokenStatusListError::MalformedReference("status"));
        };
        let Some(list) = status.get("status_list") else {
            return Ok(None);
        };
        let Some(list) = list.as_object() else {
            return Err(TokenStatusListError::MalformedReference("status_list"));
        };
        let idx = list
            .get("idx")
            .and_then(Value::as_u64)
            .ok_or(TokenStatusListError::MalformedReference("idx"))?;
        let uri = list
            .get("uri")
            .and_then(Value::as_str)
            .filter(|uri| !uri.is_empty())
            .ok_or(TokenStatusListError::MalformedReference("uri"))?;
        Ok(Some(Self {
            idx,
            uri: uri.to_owned(),
        }))
    }
}
