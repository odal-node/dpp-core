//! The Status List Token of draft-ietf-oauth-status-list-21 section 5.1, and the
//! check of section 8.3 that reads a credential's status out of one.

use std::time::Duration;

use chrono::{DateTime, Utc};
use serde_json::{Map, Value};

use dpp_crypto::jws::PublicKey;

use crate::common::jwt::jwt_part;

use super::TYP;
use super::error::TokenStatusListError;
use super::list::TokenStatusList;
use super::reference::StatusReference;
use super::status::Status;

/// A Status List Token whose signature, type and claims have been checked.
///
/// Built by [`Self::verify`], and read with [`Self::status`]. A list is
/// verified once and read for every credential that points at it, which is why
/// the two steps are apart.
#[derive(Debug, Clone, PartialEq)]
pub struct StatusListToken {
    subject: String,
    list: TokenStatusList,
    expires_at: Option<f64>,
    ttl: Option<Duration>,
}

impl StatusListToken {
    /// Check a Status List Token, fetched from the `uri` of a credential's
    /// reference, and read its list.
    ///
    /// `key` is the status issuer's verification key. Which key that is is the
    /// caller's to say, and the draft leaves it out of scope (section 11.3): for a
    /// credential whose issuer publishes its own list it is the issuer's key, and a
    /// list signed by a key nobody trusted to speak for the credential proves
    /// nothing about it. Fetching the token is I/O and not this crate's job.
    ///
    /// Checks, in order: the signature, so nothing after it reasons about claims
    /// from an unauthenticated token; `typ`; the claims section 5.1 requires, `sub`,
    /// `iat` and `status_list`; `exp` against `now`; `ttl` where present; and the
    /// list, which must declare 1, 2, 4 or 8 `bits` and decompress within
    /// [`MAX_DECODED_BYTES`](super::MAX_DECODED_BYTES). Only signatures are
    /// accepted; the draft's MAC form is not read.
    ///
    /// `now` is supplied and not read, so the result can be reproduced.
    ///
    /// # Errors
    ///
    /// See [`TokenStatusListError`].
    pub fn verify(
        token: &str,
        key: &PublicKey,
        now: DateTime<Utc>,
    ) -> Result<Self, TokenStatusListError> {
        match dpp_crypto::jws::verifier::verify_jws(token, key) {
            Ok(true) => {}
            _ => return Err(TokenStatusListError::BadSignature),
        }

        let header = jwt_part(token, 0).ok_or(TokenStatusListError::BadSignature)?;
        let typ = header
            .get("typ")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if typ != TYP {
            return Err(TokenStatusListError::WrongTyp(typ.to_owned()));
        }

        let claims = jwt_part(token, 1).ok_or(TokenStatusListError::BadClaim("payload"))?;
        let subject = claims
            .get("sub")
            .and_then(Value::as_str)
            .ok_or(TokenStatusListError::BadClaim("sub"))?
            .to_owned();
        // Required, and not compared with anything: whether a list is fresh enough
        // is a policy of the caller's (section 8.3 step 4b), who has `iat` in the
        // token they fetched.
        numeric_date(&claims, "iat")?.ok_or(TokenStatusListError::BadClaim("iat"))?;
        let expires_at = numeric_date(&claims, "exp")?;
        let ttl = ttl(&claims)?;

        let list = list_of(&claims)?;
        let token = Self {
            subject,
            list,
            expires_at,
            ttl,
        };
        token.check_not_expired(now)?;
        Ok(token)
    }

    /// The status a credential's reference points at.
    ///
    /// Checks `exp` again at `now`, so a token kept in a cache cannot answer after
    /// it has expired; then that the reference's `uri` is this token's `sub`
    /// (section 8.3 step 4a); then that `idx` is inside the list; and reads the
    /// entry.
    ///
    /// Call it after the credential itself has verified: section 8.3 asks for that
    /// first, so that a credential that fails is never looked up.
    ///
    /// # Errors
    ///
    /// See [`TokenStatusListError`]. A failure is not a statement of validity, and
    /// the credential should be refused (section 8.3: "no status statement can be
    /// made").
    pub fn status(
        &self,
        reference: &StatusReference,
        now: DateTime<Utc>,
    ) -> Result<Status, TokenStatusListError> {
        self.check_not_expired(now)?;
        if reference.uri != self.subject {
            return Err(TokenStatusListError::SubjectMismatch {
                token: self.subject.clone(),
                reference: reference.uri.clone(),
            });
        }
        let value = self
            .list
            .get(reference.idx)
            .ok_or(TokenStatusListError::IndexOutOfBounds {
                idx: reference.idx,
                entries: self.list.entries(),
            })?;
        Status::from_value(value)
    }

    /// `ttl`: how long the token may be kept before it is fetched again, when the
    /// status issuer gave one (section 8.3 step 4d). Caching is the caller's.
    #[must_use]
    pub fn ttl(&self) -> Option<Duration> {
        self.ttl
    }

    fn check_not_expired(&self, now: DateTime<Utc>) -> Result<(), TokenStatusListError> {
        match self.expires_at {
            Some(exp) if seconds(now) >= exp => Err(TokenStatusListError::Expired),
            _ => Ok(()),
        }
    }
}

/// `now` as seconds since the epoch, to compare with a NumericDate, which may be
/// fractional (RFC 7519 section 2).
fn seconds(now: DateTime<Utc>) -> f64 {
    now.timestamp() as f64 + f64::from(now.timestamp_subsec_nanos()) / 1e9
}

/// `claim` as a NumericDate: `None` when absent, a finite JSON number when present.
///
/// A malformed one is refused and not read as absent: an `exp` that did not parse
/// would otherwise become no `exp`, which is the unbounded case.
fn numeric_date(
    claims: &Map<String, Value>,
    claim: &'static str,
) -> Result<Option<f64>, TokenStatusListError> {
    match claims.get(claim) {
        None => Ok(None),
        Some(value) => value
            .as_f64()
            .filter(|seconds| seconds.is_finite())
            .map(Some)
            .ok_or(TokenStatusListError::BadClaim(claim)),
    }
}

/// `ttl`, which section 5.1 says "MUST be a positive number".
fn ttl(claims: &Map<String, Value>) -> Result<Option<Duration>, TokenStatusListError> {
    let Some(value) = claims.get("ttl") else {
        return Ok(None);
    };
    value
        .as_f64()
        .filter(|seconds| *seconds > 0.0)
        .and_then(|seconds| Duration::try_from_secs_f64(seconds).ok())
        .map(Some)
        .ok_or(TokenStatusListError::BadClaim("ttl"))
}

/// The list in the `status_list` claim: `bits` and `lst`, both required.
fn list_of(claims: &Map<String, Value>) -> Result<TokenStatusList, TokenStatusListError> {
    let object = claims
        .get("status_list")
        .and_then(Value::as_object)
        .ok_or(TokenStatusListError::BadClaim("status_list"))?;
    let bits = object
        .get("bits")
        .and_then(Value::as_u64)
        .ok_or(TokenStatusListError::BadClaim("bits"))?;
    let bits = u8::try_from(bits).map_err(|_| TokenStatusListError::UnsupportedBits(bits))?;
    let lst = object
        .get("lst")
        .and_then(Value::as_str)
        .ok_or(TokenStatusListError::BadClaim("lst"))?;
    TokenStatusList::from_encoded(bits, lst)
}
