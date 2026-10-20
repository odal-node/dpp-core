//! Error types: identifier validation ([`RegistryValidationError`]) and
//! registry-operation errors ([`EuRegistryError`] / [`EuRegistryErrorKind`]).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::basis::RegistryBasis;

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
    /// A facility identifier in a scheme other than `gln` failed the
    /// structural/checksum check for its scheme, as a malformed LEI or D-U-N-S
    /// number does. A malformed GLN is [`InvalidGln`](Self::InvalidGln).
    InvalidFacilityId { scheme: String, value: String },
    /// An operator or facility identifier whose scheme has no EN 18219 clause 6
    /// route this build can establish.
    ///
    /// EN 18219:2026 clause 6.1 has such an identifier follow one of the clause 6
    /// schemes where regulation requires one, and a registration carries both. An
    /// `eori` or a `vat` identifier is well-formed and has no ICD, so it passes
    /// [`validate`](crate::OperatorIdentifier::validate) and is refused here, which
    /// is why this is a separate error and not `InvalidOperatorId`.
    NoClause6Route {
        /// What the identifier names: `"operator"` or `"facility"`.
        subject: &'static str,
        /// The scheme string as given.
        scheme: String,
    },
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
    /// A submission carries no passports, so it would register nothing.
    EmptySubmission,
    /// A submission carries more passports than the registry accepts.
    SubmissionTooLarge {
        /// How many passports the submission carries.
        count: usize,
        /// The most one submission may carry.
        max: usize,
    },
    /// One passport in a submission failed, so **the submission** failed.
    ///
    /// The index is what makes an all-or-nothing refusal actionable: without it
    /// a caller holding a hundred passports is told only that one of them is
    /// wrong, and the registry assigns no per-passport record to point at —
    /// there are no records, which is the whole reason the refusal is total.
    SubmissionPassportInvalid {
        /// Zero-based position of the offending passport in the submission.
        index: usize,
        /// Why that passport was refused.
        source: Box<RegistryValidationError>,
    },
    /// An EN 18219 clause 5 scheme this crate has no registry scheme value for.
    ///
    /// Reachable only because `dpp_domain::identifier::ProductIdentifier` is
    /// `#[non_exhaustive]`: a match on it cannot be exhaustive from here, so a
    /// scheme added upstream and not mapped here arrives as a wildcard. Refusing
    /// is the loudest this crate can be — the alternative is labelling it with
    /// whatever string the wildcard arm happened to choose.
    UnmappedIdentifierScheme {
        /// The identifier, as [`as_str`](dpp_domain::identifier::ProductIdentifier::as_str)
        /// renders it — the only thing knowable about a variant this crate does
        /// not recognise.
        identifier: String,
    },
    /// Two passports in one submission carry the same unique product identifier.
    ///
    /// 👁️ A named refusal in the registry's User Guide (v1.03, chapter 6.2.2):
    /// each passport in a request must be uniquely identifiable. Both positions
    /// are given, because either one may be the mistake.
    DuplicateProductIdentifier {
        /// Zero-based position of the first passport carrying the identifier.
        first: usize,
        /// Zero-based position of the passport repeating it.
        second: usize,
    },
    /// The unique product identifier is not a URL the registry accepts.
    InvalidProductIdentifierUrl {
        /// The identifier as given.
        value: String,
        /// What is wrong with it.
        reason: &'static str,
    },
    /// A proof of registration says the registration happened after the proof
    /// reporting it was generated.
    ///
    /// IR (EU) 2026/1778 Art. 9(2)(d) has the proof carry the registration of the
    /// latest version *at the time it is generated*, so the order is fixed by
    /// the article, and a proof stating the reverse describes events that did not
    /// happen.
    ProofPredatesRegistration {
        /// When the proof says the passport was registered.
        registered_at: DateTime<Utc>,
        /// When the proof says it was generated.
        generated_at: DateTime<Utc>,
    },
    /// The registration contradicts the passport it registers, or leaves out a
    /// link the passport shows exists.
    ///
    /// IR (EU) 2026/1778 Art. 8(7)(b)–(c): the registry reads the passport on
    /// submission and confirms the registration data against it, and against
    /// the level it is registered at.
    Incoherent {
        /// The registration field, by its wire name.
        field: &'static str,
        /// What the registration says, or `None` where it says nothing.
        registered: Option<String>,
        /// What the passport states.
        in_passport: String,
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
            Self::InvalidFacilityId { scheme, value } => {
                write!(f, "invalid {scheme} facility identifier '{value}'")
            }
            Self::NoClause6Route { subject, scheme } => {
                write!(
                    f,
                    "the {subject} identifier scheme '{scheme}' has no EN 18219 clause 6 route in \
                     this build: a registration carries an LEI, a D-U-N-S number or a GLN"
                )
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
            Self::EmptySubmission => {
                write!(f, "a submission must carry at least one passport")
            }
            Self::SubmissionTooLarge { count, max } => {
                write!(
                    f,
                    "submission carries {count} passports; the registry accepts {max} per submission"
                )
            }
            Self::SubmissionPassportInvalid { index, source } => {
                write!(
                    f,
                    "passport at index {index} is invalid, so the whole submission is refused: {source}"
                )
            }
            Self::UnmappedIdentifierScheme { identifier } => {
                write!(
                    f,
                    "'{identifier}' is under an EN 18219 scheme this crate has no registry scheme value for"
                )
            }
            Self::DuplicateProductIdentifier { first, second } => {
                write!(
                    f,
                    "passports at index {first} and {second} carry the same unique product identifier"
                )
            }
            Self::InvalidProductIdentifierUrl { value, reason } => {
                write!(f, "unique product identifier '{value}' {reason}")
            }
            Self::ProofPredatesRegistration {
                registered_at,
                generated_at,
            } => {
                write!(
                    f,
                    "the proof was generated at {generated_at}, before the registration it reports at {registered_at}"
                )
            }
            Self::Incoherent {
                field,
                registered: Some(registered),
                in_passport,
            } => {
                write!(
                    f,
                    "the registration's {field} is '{registered}', and the passport states '{in_passport}'"
                )
            }
            Self::Incoherent {
                field,
                registered: None,
                in_passport,
            } => {
                write!(
                    f,
                    "the registration leaves out {field}, which the passport states as '{in_passport}'"
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

impl EuRegistryErrorKind {
    /// The HTTP status this kind is reported as, and what backs the choice.
    ///
    /// `None` for the kinds that describe something other than a response: a
    /// connection that never completed and a timeout have no status because no
    /// status arrived, and [`InvalidResponse`](Self::InvalidResponse) is *this
    /// crate's* verdict on a response whose own status could be anything. A
    /// caller inventing a number for those would be putting a value on the wire
    /// that no registry chose.
    ///
    /// 🚨 **Every status here is [`RegistryBasis::Assumed`] and that is not a
    /// placeholder to be quietly upgraded.** It exists so a mock and a real
    /// adapter make the *same* choice rather than each guessing — the mock being
    /// the only oracle the adapter is tested against, a disagreement between
    /// them is invisible. Agreement is all this buys; it is not knowledge of
    /// what the registry sends. The one status anyone has seen is
    /// [`STATUS_IDEMPOTENCY_KEY_REUSED`].
    ///
    /// Matched exhaustively on purpose. The enum is `#[non_exhaustive]` to
    /// downstream, but inside this crate a new kind must be given a status here
    /// or this stops compiling — a wildcard arm would hand it one silently.
    #[must_use]
    pub fn http_status(&self) -> Option<(u16, RegistryBasis)> {
        let status = match self {
            Self::Unauthorized => 401,
            Self::RegistrationRejected => 422,
            Self::RateLimited => 429,
            Self::NotFound => 404,
            Self::RegistryInternalError => 500,
            Self::ConnectionFailed | Self::Timeout | Self::InvalidResponse => return None,
        };
        Some((status, RegistryBasis::Assumed))
    }
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
/// 👁️ **Observed** in the registry's web client (first 2026-09-16, again
/// 2026-10-14), which reads
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

/// The HTTP status accompanying [`SUB_CODE_IDEMPOTENCY_KEY_REUSED`].
///
/// 👁️ **The only status in this crate that anyone has actually seen.** Every
/// other one is [`EuRegistryErrorKind::http_status`]'s, and is ours. Stated as a
/// constant rather than left inside that mapping because it does not belong to
/// any [`EuRegistryErrorKind`] — a replayed key is reported by the body's
/// `subCode`, and there is no `Conflict` kind for it to hang on.
pub const STATUS_IDEMPOTENCY_KEY_REUSED: u16 = 409;

/// What backs [`STATUS_IDEMPOTENCY_KEY_REUSED`]: the client's check of a 409
/// against this sub-code, read on the date the client was last read.
pub const STATUS_IDEMPOTENCY_KEY_REUSED_BASIS: RegistryBasis = RegistryBasis::Observed {
    on: crate::endpoint::LAST_READ,
};

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
