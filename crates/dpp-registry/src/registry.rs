//! EU Central Registry types for ESPR Article 13 compliance.
//!
//! These types model the data exchanged with the EU EUDPP Central Registry.
//! The actual HTTP transport is implemented in the platform repo; this crate
//! only defines the shapes.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Identifier validation ─────────────────────────────────────────────────

/// Error returned when a bridge identifier fails structural validation.
#[derive(Debug, Clone, PartialEq, Eq)]
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
        }
    }
}

impl std::error::Error for RegistryValidationError {}

/// Validate an ISO 3166-1 alpha-2 country code.
///
/// Membership is checked against the officially assigned set (via
/// [`dpp_rules::country_code_valid`]), so shape-valid but unassigned codes such
/// as `XX`/`QZ` are rejected. `EU` is reserved by the EC but is not an assigned
/// ISO 3166-1 alpha-2 code, so it is rejected too.
fn validate_country_code(code: &str) -> Result<(), RegistryValidationError> {
    if code.is_empty() {
        return Ok(()); // unknown/not-yet-set is acceptable pre-go-live
    }
    if dpp_rules::country_code_valid(code) {
        Ok(())
    } else {
        Err(RegistryValidationError::InvalidCountryCode {
            code: code.to_owned(),
        })
    }
}

// ─── Persistent identifiers (Article 13) ────────────────────────────────────

/// Unique product identifier — identifies the product model/type.
///
/// Typically a GTIN-14 or similar standardised product code. The EU registry
/// uses this to group all items of the same product under one entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProductIdentifier {
    /// The identifier scheme (e.g. `"gtin"`, `"gln"`, `"did"`).
    pub scheme: String,
    /// The identifier value (e.g. `"09506000134352"`).
    pub value: String,
    /// Optional human-readable label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

impl ProductIdentifier {
    /// Validate the identifier by scheme.
    ///
    /// - `"gtin"`: value must be a structurally valid GTIN-14 (14 digits, mod-10 check).
    /// - Other schemes: no structural validation (formats vary; validate when EU spec is published).
    pub fn validate(&self) -> Result<(), RegistryValidationError> {
        if self.scheme == "gtin" {
            dpp_domain::Gtin::parse(&self.value)
                .map(|_| ())
                .map_err(|e| RegistryValidationError::InvalidGtin {
                    value: self.value.clone(),
                    reason: format!("{e:?}"),
                })?;
        }
        Ok(())
    }
}

/// Product item identifier — identifies an individual serialised unit.
///
/// For serialised products (batteries with serial numbers, individual garments
/// with SGTIN), this distinguishes one physical item from another.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProductItemIdentifier {
    /// The identifier scheme (e.g. `"sgtin"`, `"serial"`, `"batch+serial"`).
    pub scheme: String,
    /// The identifier value.
    pub value: String,
    /// Batch or lot number, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_id: Option<String>,
}

/// Facility identifier — identifies the manufacturing or assembly facility.
///
/// Used for market surveillance to trace products back to their physical
/// origin. Must match the facility registered under the economic operator.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FacilityIdentifier {
    /// The facility code scheme (e.g. `"gln"`, `"lei"`, `"national"`).
    pub scheme: String,
    /// The facility identifier value.
    pub value: String,
    /// Human-readable facility name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// ISO 3166-1 alpha-2 country code where the facility is located.
    pub country: String,
    /// Street address or location description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
}

impl FacilityIdentifier {
    /// Validate the facility identifier.
    ///
    /// Checks the country code and, when `scheme == "gln"`, that the value is a
    /// structurally valid GS1 GLN (13 digits, mod-10 check). Other schemes
    /// (`"lei"`, `"national"`, …) are not structurally verified here.
    pub fn validate(&self) -> Result<(), RegistryValidationError> {
        validate_country_code(&self.country)?;
        if self.scheme == "gln" {
            dpp_domain::Gln::parse(&self.value)
                .map(|_| ())
                .map_err(|e| RegistryValidationError::InvalidGln {
                    value: self.value.clone(),
                    reason: format!("{e:?}"),
                })?;
        }
        Ok(())
    }
}

/// Economic operator identifier — identifies the responsible legal entity.
///
/// This is the entity accountable for the DPP under ESPR. When a transfer
/// of responsibility occurs, the registry record is updated to reflect the
/// new operator.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OperatorIdentifier {
    /// The operator identifier scheme (e.g. `"vat"`, `"lei"`, `"eori"`, `"did"`).
    pub scheme: String,
    /// The identifier value (e.g. EU VAT number, LEI code).
    pub value: String,
    /// Legal entity name.
    pub name: String,
    /// ISO 3166-1 alpha-2 country of registration.
    pub country: String,
    /// DID of the operator, if available (for VC-based authentication).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub did: Option<String>,
}

impl OperatorIdentifier {
    /// Validate the operator identifier.
    ///
    /// Checks the country code first, then applies a per-scheme structural or
    /// checksum check (see `validate_operator_scheme`).
    pub fn validate(&self) -> Result<(), RegistryValidationError> {
        validate_country_code(&self.country)?;
        validate_operator_scheme(&self.scheme, &self.value)
    }
}

/// Per-scheme structural/checksum validation of an economic-operator identifier.
///
/// - `lei`  — ISO 17442: 20 alphanumerics with an ISO 7064 MOD 97-10 check digit.
/// - `eori` — 2-letter country prefix + 1..=15 alphanumerics.
/// - `vat`  — 2-letter country prefix + alphanumerics (member-state check digits
///   are **not** enforced; formats vary).
/// - `duns` — exactly 9 digits.
/// - `did`  — a DID; not structurally verified here (the crypto layer validates it).
/// - any other scheme — accepted but **not** structurally verified.
fn validate_operator_scheme(scheme: &str, value: &str) -> Result<(), RegistryValidationError> {
    let ok = match scheme {
        "lei" => lei_checksum_valid(value),
        "eori" => has_country_prefix(value, 15),
        "vat" => has_country_prefix(value, usize::MAX),
        "duns" => value.len() == 9 && value.bytes().all(|b| b.is_ascii_digit()),
        // "did" and unknown schemes are accepted without structural verification.
        _ => true,
    };
    if ok {
        Ok(())
    } else {
        Err(RegistryValidationError::InvalidOperatorId {
            scheme: scheme.to_owned(),
            value: value.to_owned(),
        })
    }
}

/// ISO 7064 MOD 97-10 check over a 20-character LEI (ISO 17442).
///
/// Letters map A→10 … Z→35; the streamed value mod 97 must equal 1.
fn lei_checksum_valid(s: &str) -> bool {
    if s.len() != 20
        || !s
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
    {
        return false;
    }
    let mut rem: u32 = 0;
    for b in s.bytes() {
        let v = if b.is_ascii_digit() {
            u32::from(b - b'0')
        } else {
            u32::from(b - b'A') + 10
        };
        // A letter contributes two decimal positions (10–35); a digit one.
        rem = if v >= 10 {
            (rem * 100 + v) % 97
        } else {
            (rem * 10 + v) % 97
        };
    }
    rem == 1
}

/// Two-letter country prefix followed by 1..=`max_body` alphanumeric characters.
fn has_country_prefix(s: &str, max_body: usize) -> bool {
    let b = s.as_bytes();
    s.len() >= 3
        && b[0].is_ascii_uppercase()
        && b[1].is_ascii_uppercase()
        && (s.len() - 2) <= max_body
        && s[2..]
            .bytes()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
}

// ─── Registration payload ───────────────────────────────────────────────────

/// The full data payload sent to the EU registry when registering a DPP.
///
/// Contains all four persistent identifiers plus metadata about the passport.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RegistrationPayload {
    /// Internal passport UUID (Odal Node's identifier).
    pub passport_id: Uuid,
    /// The product identifier (GTIN, etc.).
    pub product_id: ProductIdentifier,
    /// The individual item identifier (serial, batch+serial, etc.).
    pub item_id: ProductItemIdentifier,
    /// The manufacturing facility identifier.
    pub facility_id: FacilityIdentifier,
    /// The responsible economic operator identifier.
    pub operator_id: OperatorIdentifier,
    /// EU ESPR sector code (e.g. `"textile"`, `"battery"`).
    pub sector: String,
    /// Schema version of the DPP data (e.g. `"1.1.0"`).
    pub schema_version: String,
    /// The GS1 Digital Link URL resolving to this DPP.
    pub digital_link_url: String,
    /// ISO 8601 timestamp when the DPP was first published.
    pub published_at: DateTime<Utc>,
    /// JWS signature of the DPP data (for integrity verification by the registry).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jws_signature: Option<String>,
}

impl RegistrationPayload {
    /// Validate all four Article-13 identifiers and required fields.
    ///
    /// Call before sending to the EU registry to catch structural errors
    /// (GTIN checksum, invalid country codes) before a network round-trip.
    pub fn validate(&self) -> Result<(), RegistryValidationError> {
        self.product_id.validate()?;
        self.facility_id.validate()?;
        self.operator_id.validate()?;
        if self.digital_link_url.is_empty() {
            return Err(RegistryValidationError::MissingRequiredField(
                "digitalLinkUrl".into(),
            ));
        }
        Ok(())
    }
}

// ─── Registry envelope ──────────────────────────────────────────────────────

/// Wrapper envelope for all requests to the EU registry.
///
/// Includes authentication metadata and the payload. The actual authentication
/// mechanism (OIDC, mTLS, etc.) is specified by the EU and handled by the
/// platform's HTTP adapter.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EuRegistryEnvelope {
    /// API version of the registry protocol (e.g. `"1.0"`).
    pub api_version: String,
    /// Unique request ID for idempotency and tracing.
    pub request_id: Uuid,
    /// ISO 8601 timestamp of when the request was created.
    pub timestamp: DateTime<Utc>,
    /// The registration payload.
    pub payload: RegistrationPayload,
}

// ─── Registry response ──────────────────────────────────────────────────────

/// Status codes returned by the EU registry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RegistryStatusCode {
    /// Registration received and being processed.
    Pending,
    /// Successfully registered — DPP is live in the EU registry.
    Registered,
    /// Registration rejected — see `rejection_reasons`.
    Rejected,
    /// Registration suspended by a market surveillance authority.
    SuspendedByAuthority,
    /// DPP has been deactivated (product withdrawn from market).
    Deactivated,
}

/// Response from the EU registry after a registration or status query.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EuRegistryResponse {
    /// The registry's own unique ID for this DPP registration.
    pub registry_id: String,
    /// The passport ID that was registered.
    pub passport_id: Uuid,
    /// Current status in the registry.
    pub status: RegistryStatusCode,
    /// Human-readable status message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Rejection reasons, if status is `Rejected`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejection_reasons: Option<Vec<String>>,
    /// Timestamp when the registry last updated this record.
    pub updated_at: DateTime<Utc>,
}

/// Simplified status query response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StatusResponse {
    pub registry_id: String,
    pub status: RegistryStatusCode,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

// ─── Transfer notification ──────────────────────────────────────────────────

/// Notification sent to the EU registry when a transfer of responsibility occurs.
///
/// 🟠 COMPLIANCE-PIN PENDING: checked against the verbatim OJ text (Regulation (EU)
/// 2024/1781) — there is **no distinct "transfer of responsibility" provision** by
/// that name in Articles 9-15. The closest support is the general data-accuracy
/// duty ("the data in the digital product passport shall be accurate, complete and
/// up to date", **Art. 9(1)**) plus the registry-upload duty (**Art. 13(4)**); a
/// dedicated transfer-notice obligation is not textually confirmed. The prior
/// single-article "Article 9" citation is corrected to this honest, narrower basis
/// — this notification is a sound compliance-hygiene design, not a verbatim-cited
/// requirement.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TransferNotification {
    /// The passport being transferred.
    pub passport_id: Uuid,
    /// The registry's ID for this DPP.
    pub registry_id: String,
    /// The operator transferring responsibility.
    pub from_operator: OperatorIdentifier,
    /// The operator receiving responsibility.
    pub to_operator: OperatorIdentifier,
    /// Reason for the transfer (maps to `TransferReason` in dpp-domain).
    pub reason: String,
    /// ISO 8601 timestamp of the transfer.
    pub transferred_at: DateTime<Utc>,
    /// JWS signature from the outgoing operator.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_signature: Option<String>,
    /// JWS signature from the incoming operator.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_signature: Option<String>,
}

// ─── Error types ────────────────────────────────────────────────────────────

/// Error categories for EU registry operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
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

// ─── Registry endpoint configuration ────────────────────────────────────────

/// Known EU registry authority types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RegistryAuthority {
    /// EU Central DPP Registry (production).
    EuCentral,
    /// EU Sandbox / test environment.
    EuSandbox,
    /// National registry (member state specific).
    National(String),
}

/// Configuration for connecting to a specific EU registry endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RegistryEndpoint {
    /// Which authority this endpoint belongs to.
    pub authority: RegistryAuthority,
    /// Base URL of the registry API.
    pub base_url: String,
    /// API version supported (e.g. `"1.0"`).
    pub api_version: String,
    /// Whether mTLS is required.
    pub mtls_required: bool,
    /// OAuth2 / OIDC token endpoint, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_endpoint: Option<String>,
}

impl RegistryEndpoint {
    /// Create a sandbox endpoint for development/testing.
    pub fn sandbox() -> Self {
        Self {
            authority: RegistryAuthority::EuSandbox,
            // ⚠️ COMPLIANCE-PIN PENDING (watchlist 🟠): sandbox URL is an educated guess
            // based on the EC's EUDPP work programme. Confirm against the published sandbox
            // spec before enabling live calls. Track: ESPR implementing acts / DG GROW.
            base_url: "https://sandbox.eudpp-registry.europa.eu/api/v1".into(),
            // ⚠️ COMPLIANCE-PIN PENDING (watchlist 🟠): api_version "1.0" is provisional.
            // Update when the EU publishes the registry API specification.
            api_version: "1.0".into(),
            mtls_required: false,
            token_endpoint: Some("https://sandbox.eudpp-registry.europa.eu/oauth2/token".into()),
        }
    }

    /// Create a production endpoint.
    ///
    /// ⚠️ **PROVISIONAL**: The EU Central DPP Registry API has not been published
    /// as of 2026-06. All URLs, `api_version`, and auth flows are educated guesses
    /// based on the ESPR implementing acts and DG GROW work programme. Do NOT point
    /// this at real products until the Commission publishes the final spec and these
    /// constants are confirmed (COMPLIANCE-PIN PENDING).
    pub fn production() -> Self {
        Self {
            authority: RegistryAuthority::EuCentral,
            // ⚠️ COMPLIANCE-PIN PENDING (watchlist 🟠): placeholder URL — confirm
            // the real production endpoint from the published EU registry API spec.
            base_url: "https://eudpp-registry.europa.eu/api/v1".into(),
            api_version: "1.0".into(),
            mtls_required: true,
            token_endpoint: Some("https://eudpp-registry.europa.eu/oauth2/token".into()),
        }
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_product_id() -> ProductIdentifier {
        ProductIdentifier {
            scheme: "gtin".into(),
            value: "09506000134352".into(),
            label: Some("Organic Cotton T-Shirt".into()),
        }
    }

    fn sample_item_id() -> ProductItemIdentifier {
        ProductItemIdentifier {
            scheme: "sgtin".into(),
            value: "09506000134352.21.ABC123".into(),
            batch_id: Some("BATCH-2026-Q2-001".into()),
        }
    }

    fn sample_facility_id() -> FacilityIdentifier {
        FacilityIdentifier {
            scheme: "gln".into(),
            value: "4012345000009".into(),
            name: Some("Dhaka Manufacturing Unit 3".into()),
            country: "BD".into(),
            address: Some("123 Industrial Zone, Gazipur".into()),
        }
    }

    fn sample_operator_id() -> OperatorIdentifier {
        OperatorIdentifier {
            scheme: "vat".into(),
            value: "DE123456789".into(),
            name: "EcoTextile GmbH".into(),
            country: "DE".into(),
            did: Some("did:web:ecotextile.de".into()),
        }
    }

    fn sample_payload() -> RegistrationPayload {
        RegistrationPayload {
            passport_id: Uuid::nil(),
            product_id: sample_product_id(),
            item_id: sample_item_id(),
            facility_id: sample_facility_id(),
            operator_id: sample_operator_id(),
            sector: "textile".into(),
            schema_version: "1.1.0".into(),
            digital_link_url: "https://id.ecotextile.de/01/09506000134352/21/ABC123".into(),
            published_at: Utc::now(),
            jws_signature: Some("eyJhbGciOiJFZERTQSJ9...".into()),
        }
    }

    #[test]
    fn registration_payload_round_trip() {
        let payload = sample_payload();
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["sector"], "textile");
        assert_eq!(json["productId"]["scheme"], "gtin");
        assert_eq!(json["operatorId"]["country"], "DE");
        let back: RegistrationPayload = serde_json::from_value(json).unwrap();
        assert_eq!(payload.passport_id, back.passport_id);
        assert_eq!(payload.product_id, back.product_id);
    }

    #[test]
    fn envelope_round_trip() {
        let envelope = EuRegistryEnvelope {
            api_version: "1.0".into(),
            request_id: Uuid::nil(),
            timestamp: Utc::now(),
            payload: sample_payload(),
        };
        let json = serde_json::to_string(&envelope).unwrap();
        let back: EuRegistryEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(envelope.api_version, back.api_version);
    }

    #[test]
    fn response_with_rejection() {
        let response = EuRegistryResponse {
            registry_id: "EU-REG-2026-00001".into(),
            passport_id: Uuid::nil(),
            status: RegistryStatusCode::Rejected,
            message: Some("Validation failed".into()),
            rejection_reasons: Some(vec![
                "Product identifier scheme 'custom' not recognized".into(),
                "Facility country 'XX' is not a valid ISO 3166-1 code".into(),
            ]),
            updated_at: Utc::now(),
        };
        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["status"], "rejected");
        assert_eq!(json["rejectionReasons"].as_array().unwrap().len(), 2);
        let back: EuRegistryResponse = serde_json::from_value(json).unwrap();
        assert_eq!(back.status, RegistryStatusCode::Rejected);
    }

    #[test]
    fn transfer_notification_round_trip() {
        let notif = TransferNotification {
            passport_id: Uuid::nil(),
            registry_id: "EU-REG-2026-00001".into(),
            from_operator: sample_operator_id(),
            to_operator: OperatorIdentifier {
                scheme: "vat".into(),
                value: "FR987654321".into(),
                name: "ModeVerte SARL".into(),
                country: "FR".into(),
                did: Some("did:web:modeverte.fr".into()),
            },
            reason: "sale".into(),
            transferred_at: Utc::now(),
            from_signature: Some("sig_from...".into()),
            to_signature: Some("sig_to...".into()),
        };
        let json = serde_json::to_value(&notif).unwrap();
        assert_eq!(json["reason"], "sale");
        assert_eq!(json["toOperator"]["name"], "ModeVerte SARL");
        let back: TransferNotification = serde_json::from_value(json).unwrap();
        assert_eq!(notif.registry_id, back.registry_id);
    }

    #[test]
    fn error_display() {
        let err = EuRegistryError {
            kind: EuRegistryErrorKind::RegistrationRejected,
            message: "missing facility identifier".into(),
            status_code: Some(422),
            registry_error_code: Some("ERR_MISSING_FACILITY".into()),
        };
        let display = format!("{err}");
        assert!(display.contains("RegistrationRejected"));
        assert!(display.contains("missing facility identifier"));
    }

    #[test]
    fn sandbox_endpoint() {
        let ep = RegistryEndpoint::sandbox();
        assert_eq!(ep.authority, RegistryAuthority::EuSandbox);
        assert!(!ep.mtls_required);
        assert!(ep.base_url.contains("sandbox"));
    }

    #[test]
    fn production_endpoint() {
        let ep = RegistryEndpoint::production();
        assert_eq!(ep.authority, RegistryAuthority::EuCentral);
        assert!(ep.mtls_required);
    }

    #[test]
    fn status_response_round_trip() {
        let status = StatusResponse {
            registry_id: "EU-REG-2026-00001".into(),
            status: RegistryStatusCode::Registered,
            updated_at: Utc::now(),
            message: None,
        };
        let json = serde_json::to_string(&status).unwrap();
        let back: StatusResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(back.status, RegistryStatusCode::Registered);
    }

    // ── B1 validation tests ─────────────────────────────────────────────────

    #[test]
    fn valid_gtin_product_identifier_passes() {
        let id = ProductIdentifier {
            scheme: "gtin".into(),
            value: "09506000134352".into(),
            label: None,
        };
        assert!(id.validate().is_ok());
    }

    #[test]
    fn invalid_gtin_product_identifier_fails() {
        let id = ProductIdentifier {
            scheme: "gtin".into(),
            value: "12345678901234".into(), // bad check digit
            label: None,
        };
        assert!(matches!(
            id.validate(),
            Err(RegistryValidationError::InvalidGtin { .. })
        ));
    }

    #[test]
    fn non_gtin_scheme_skips_checksum_validation() {
        let id = ProductIdentifier {
            scheme: "passport_id".into(),
            value: "not-a-gtin-at-all".into(),
            label: None,
        };
        assert!(id.validate().is_ok());
    }

    #[test]
    fn valid_iso_country_passes() {
        let fac = FacilityIdentifier {
            scheme: "gln".into(),
            value: "4012345000009".into(),
            name: None,
            country: "DE".into(),
            address: None,
        };
        assert!(fac.validate().is_ok());
    }

    #[test]
    fn empty_country_passes_as_unknown() {
        let fac = FacilityIdentifier {
            scheme: "national".into(),
            value: "FAC-001".into(),
            name: None,
            country: String::new(),
            address: None,
        };
        assert!(fac.validate().is_ok());
    }

    #[test]
    fn gln_facility_bad_check_digit_rejected() {
        let fac = FacilityIdentifier {
            scheme: "gln".into(),
            value: "4000001000002".into(), // shape-valid but wrong GS1 check digit
            name: None,
            country: "DE".into(),
            address: None,
        };
        assert!(matches!(
            fac.validate(),
            Err(RegistryValidationError::InvalidGln { .. })
        ));
    }

    #[test]
    fn lei_operator_checksum_validated() {
        let valid = OperatorIdentifier {
            scheme: "lei".into(),
            value: "5493001KJTIIGC8Y1R12".into(), // valid ISO 7064 MOD 97-10
            name: "Example AG".into(),
            country: "DE".into(),
            did: None,
        };
        assert!(valid.validate().is_ok());

        let bad = OperatorIdentifier {
            value: "969500GU3KE7GR9NDV41".into(), // wrong check digits
            ..valid
        };
        assert!(matches!(
            bad.validate(),
            Err(RegistryValidationError::InvalidOperatorId { .. })
        ));
    }

    #[test]
    fn duns_and_eori_structure_validated() {
        let duns_ok = OperatorIdentifier {
            scheme: "duns".into(),
            value: "150483782".into(),
            name: "X".into(),
            country: "US".into(),
            did: None,
        };
        assert!(duns_ok.validate().is_ok());

        let duns_bad = OperatorIdentifier {
            value: "15048378".into(), // 8 digits
            ..duns_ok.clone()
        };
        assert!(duns_bad.validate().is_err());

        let eori_ok = OperatorIdentifier {
            scheme: "eori".into(),
            value: "DE1234567890".into(),
            ..duns_ok.clone()
        };
        assert!(eori_ok.validate().is_ok());

        let eori_bad = OperatorIdentifier {
            scheme: "eori".into(),
            value: "1234567890".into(), // missing 2-letter country prefix
            ..duns_ok
        };
        assert!(eori_bad.validate().is_err());
    }

    #[test]
    fn unknown_operator_scheme_not_structurally_verified() {
        let op = OperatorIdentifier {
            scheme: "custom".into(),
            value: "anything-goes".into(),
            name: "X".into(),
            country: "DE".into(),
            did: None,
        };
        assert!(op.validate().is_ok());
    }

    #[test]
    fn eu_pseudo_code_rejected() {
        let op = OperatorIdentifier {
            scheme: "did".into(),
            value: "did:web:acme.example.com".into(),
            name: "ACME".into(),
            country: "EU".into(),
            did: None,
        };
        assert!(matches!(
            op.validate(),
            Err(RegistryValidationError::InvalidCountryCode { .. })
        ));
    }

    #[test]
    fn lowercase_country_rejected() {
        let op = OperatorIdentifier {
            scheme: "vat".into(),
            value: "DE123456789".into(),
            name: "Test".into(),
            country: "de".into(),
            did: None,
        };
        assert!(matches!(
            op.validate(),
            Err(RegistryValidationError::InvalidCountryCode { .. })
        ));
    }

    #[test]
    fn valid_payload_passes_validation() {
        assert!(sample_payload().validate().is_ok());
    }

    #[test]
    fn payload_with_empty_digital_link_fails() {
        let mut payload = sample_payload();
        payload.digital_link_url = String::new();
        assert!(matches!(
            payload.validate(),
            Err(RegistryValidationError::MissingRequiredField(_))
        ));
    }

    #[test]
    fn payload_with_invalid_gtin_fails() {
        let mut payload = sample_payload();
        payload.product_id.value = "99999999999999".into(); // bad check digit
        assert!(matches!(
            payload.validate(),
            Err(RegistryValidationError::InvalidGtin { .. })
        ));
    }

    #[test]
    fn validation_error_display_messages() {
        let gtin = RegistryValidationError::InvalidGtin {
            value: "123".into(),
            reason: "too short".into(),
        };
        assert_eq!(gtin.to_string(), "invalid GTIN '123': too short");

        let country = RegistryValidationError::InvalidCountryCode { code: "EU".into() };
        assert!(country.to_string().starts_with("invalid country code 'EU'"));

        let missing = RegistryValidationError::MissingRequiredField("passportId".into());
        assert_eq!(missing.to_string(), "required field 'passportId' is empty");

        // Error trait object is usable (covers the std::error::Error impl).
        let boxed: Box<dyn std::error::Error> = Box::new(gtin);
        assert!(!boxed.to_string().is_empty());
    }
}
