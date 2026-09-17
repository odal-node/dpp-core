//! Error types: identifier validation ([`RegistryValidationError`]) and
//! registry-operation errors ([`EuRegistryError`] / [`EuRegistryErrorKind`]).

use serde::{Deserialize, Serialize};

/// Error returned when a bridge identifier fails structural validation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RegistryValidationError {
    /// A GTIN value is structurally invalid (wrong length or failed mod-10).
    InvalidGtin { value: String, reason: String },
    /// A GLN facility value is structurally invalid (wrong length or failed mod-10).
    InvalidGln { value: String, reason: String },
    /// An operator identifier failed the structural/checksum check for its scheme.
    InvalidOperatorId { scheme: String, value: String },
    /// A country code is not a valid ISO 3166-1 alpha-2 code.
    InvalidCountryCode { code: String },
    /// A required payload field is empty.
    MissingRequiredField(String),
    /// An identifier finer than the declared registration level travels with
    /// the payload — e.g. a batch identifier on a model-level registration.
    GranularityMismatch {
        granularity: &'static str,
        identifier: &'static str,
    },
    /// The commodity code is not structurally a tariff code (HS-6/CN-8/TARIC-10).
    InvalidCommodityCode { value: String },
    /// A declared back-up URL is not `https://`.
    InsecureBackupUrl { value: String },
    /// The unique product identifier is longer than the registry accepts.
    ///
    /// The bound is the registry's, not ours — see
    /// [`MAX_PRODUCT_IDENTIFIER_CHARS`](crate::MAX_PRODUCT_IDENTIFIER_CHARS),
    /// including why it is worth re-checking.
    ProductIdentifierTooLong {
        /// How long the identifier actually is.
        chars: usize,
        /// The most the registry accepts.
        max: usize,
    },
}

impl std::fmt::Display for RegistryValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidGtin { value, reason } => {
                write!(f, "invalid GTIN '{value}': {reason}")
            }
            Self::InvalidGln { value, reason } => {
                write!(f, "invalid GLN '{value}': {reason}")
            }
            Self::InvalidOperatorId { scheme, value } => {
                write!(f, "invalid {scheme} operator identifier '{value}'")
            }
            Self::InvalidCountryCode { code } => {
                write!(
                    f,
                    "invalid country code '{code}': must be an ISO 3166-1 alpha-2 code (2 uppercase ASCII letters)"
                )
            }
            Self::MissingRequiredField(field) => {
                write!(f, "required field '{field}' is empty")
            }
            Self::GranularityMismatch {
                granularity,
                identifier,
            } => {
                write!(
                    f,
                    "a '{granularity}'-level registration must not carry a '{identifier}'"
                )
            }
            Self::InvalidCommodityCode { value } => {
                write!(
                    f,
                    "invalid commodity code '{value}': expected 6 (HS), 8 (CN) or 10 (TARIC) digits"
                )
            }
            Self::InsecureBackupUrl { value } => {
                write!(f, "back-up URL '{value}' must be https://")
            }
            Self::ProductIdentifierTooLong { chars, max } => {
                write!(
                    f,
                    "unique product identifier is {chars} characters; the registry accepts {max}"
                )
            }
        }
    }
}

impl std::error::Error for RegistryValidationError {}

/// Error categories for EU registry operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum EuRegistryErrorKind {
    /// Network or transport failure.
    ConnectionFailed,
    /// Authentication or authorisation failure.
    Unauthorized,
    /// The registry returned an unexpected response format.
    InvalidResponse,
    /// The registration was rejected by the registry.
    RegistrationRejected,
    /// Rate limit exceeded.
    RateLimited,
    /// The passport was not found in the registry.
    NotFound,
    /// The registry reported an internal error.
    RegistryInternalError,
    /// Request timed out.
    Timeout,
}

/// Error returned by EU registry operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EuRegistryError {
    pub kind: EuRegistryErrorKind,
    pub message: String,
    /// HTTP status code, if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,
    /// Registry-specific error code, if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry_error_code: Option<String>,
}

impl std::fmt::Display for EuRegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EU Registry error ({:?}): {}", self.kind, self.message)
    }
}

impl std::error::Error for EuRegistryError {}

/// An error body returned by the registry.
///
/// 👁️ **Observed 2026-09-16** in the registry's web client, which reads
/// `subCode` to recognise a replayed idempotency key and `traceId` to report a
/// failure to the helpdesk. Both are therefore fields the registry really
/// sends; the rest of the body's shape is unknown, which is why this models
/// only those two and carries `message` for anything human-readable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryErrorBody {
    /// A machine-readable discriminator beneath the HTTP status.
    ///
    /// The only value known by name is
    /// [`SUB_CODE_IDEMPOTENCY_KEY_REUSED`]. A `String` rather than an enum,
    /// because a closed set built from one observed member would be a guess
    /// wearing a type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_code: Option<String>,
    /// The registry's own trace identifier, quotable to the helpdesk.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    /// Anything human-readable the registry supplied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// The `subCode` accompanying **409** when an idempotency key is replayed.
///
/// 👁️ Observed in the web client, which treats this case as *already done*: it
/// keeps the key rather than minting a new one. A caller seeing it should read
/// the original submission's outcome, not resubmit.
pub const SUB_CODE_IDEMPOTENCY_KEY_REUSED: &str = "CONFLICT_IDEMPOTENCY_KEY_ALREADY_USED";

impl RegistryErrorBody {
    /// Whether this body reports a replayed idempotency key.
    #[must_use]
    pub fn is_idempotency_key_reused(&self) -> bool {
        self.sub_code.as_deref() == Some(SUB_CODE_IDEMPOTENCY_KEY_REUSED)
    }
}
