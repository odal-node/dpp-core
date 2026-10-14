//! [`REGISTRATION_ERROR_CODES`] — the codes a registration can be refused with.

/// Every registration error code the registry's web client knows a wording
/// for.
///
/// 👁️ **Observed** on 2026-10-14, as the keys under
/// `dpp.registration.errorCode` in the client's English translations
/// (`assets/i18n/en.json`). That is the client's list, not a published one: a
/// code outside it can arrive, and
/// [`PassportResult::error_code`](crate::PassportResult::error_code) carries it
/// regardless. The list is here so that a consumer, and a stand-in, use the
/// registry's own codes rather than inventing near-misses of them.
///
/// Several bear on how a consumer behaves rather than on what it shows:
/// `NOT_RESOLVABLE_UPI`, `TOO_MANY_UPI_REDIRECTS`,
/// `UPI_REDIRECT_SCHEME_DOWNGRADE` and `UPI_RESPONSE_TOO_LARGE` say the
/// registry fetches the passport at its identifier, as IR (EU) 2026/1778
/// Art. 8(7) has it read the passport content; `ORGANISATION_NOT_ACTIVE` is the
/// Art. 4(4) lapse of verified status; and `NON_UNIQUE_UPI` and
/// `DUPLICATE_UPI_IN_REGISTRY` say an identifier already registered is
/// refused, so a new version of a passport is not a second registration.
pub const REGISTRATION_ERROR_CODES: &[&str] = &[
    "UNSUPPORTED_FILE_FORMAT",
    "INVALID_FILE_SCHEMA",
    "FILE_SIZE_EXCEEDED",
    "INVALID_FILENAME",
    "ORGANISATION_NOT_ACTIVE",
    "DUPLICATE_UPI_IN_REQUEST",
    "SEMANTIC_VALIDATION_FAILED",
    "TECHNICAL_ERROR",
    "NOT_RESOLVABLE_UPI",
    "INVALID_UPI_URL",
    "DISALLOWED_UPI_URL_HOST",
    "DISALLOWED_UPI_URL_PORT",
    "UNSUPPORTED_UPI_URL_SCHEME",
    "TOO_MANY_UPI_REDIRECTS",
    "UPI_REDIRECT_SCHEME_DOWNGRADE",
    "UPI_RESPONSE_TOO_LARGE",
    "VALIDATION_RETRY_EXHAUSTED",
    "SEMANTIC_VALIDATION_UNAVAILABLE",
    "INCONSISTENT_UPI_DATA_FORMAT",
    "NON_UNIQUE_UPI",
    "DUPLICATE_UPI_IN_REGISTRY",
    "MAX_ITEMS_EXCEEDED",
];
