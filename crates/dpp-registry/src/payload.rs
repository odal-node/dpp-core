//! [`RegistrationPayload`] and its [`EuRegistryEnvelope`] wrapper.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::error::RegistryValidationError;
use super::granularity::{Granularity, RegistrationLevel};
use super::identifiers::{
    FacilityIdentifier, OperatorIdentifier, ProductIdentifier, ProductItemIdentifier,
};

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
    /// The level this passport is registered at, and the higher-level model and
    /// batch identifiers it links — IR (EU) 2026/1778 Art. 8(1), 8(4), 8(5).
    pub level: RegistrationLevel,
    /// The individual item identifier (serial, batch+serial, etc.).
    ///
    /// Required at item level and meaningless above it: a model- or batch-level
    /// registration covers every unit it groups, so naming one contradicts the
    /// Art. 8(1) level the registry checks on submission (Art. 8(7)(c)).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_id: Option<ProductItemIdentifier>,
    /// The manufacturing facility identifier.
    pub facility_id: FacilityIdentifier,
    /// The responsible economic operator identifier.
    pub operator_id: OperatorIdentifier,
    /// EU ESPR product group code (e.g. `"textile"`, `"battery"`).
    pub product_group: String,
    /// Schema version of the DPP data (e.g. `"1.1.0"`).
    pub schema_version: String,
    /// The GS1 Digital Link URL resolving to this DPP.
    pub digital_link_url: String,
    /// ISO 8601 timestamp when the DPP was first published.
    pub published_at: DateTime<Utc>,
    /// JWS signature of the DPP data (for integrity verification by the registry).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jws_signature: Option<String>,
    /// Customs tariff classification (HS-6, CN-8 or TARIC-10).
    ///
    /// Registration data the registry stores, and verifies "where relevant"
    /// against the ranges its product group permits — a check this crate cannot
    /// perform, because the ranges live in the applicable delegated act. What is
    /// checkable here is that the code is structurally a tariff code at all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commodity_code: Option<String>,
    /// Public URL of a back-up of this passport, hosted independently of the
    /// issuing node. Verified by the registry where one is declared.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backup_url: Option<String>,
}

impl RegistrationPayload {
    /// Validate all four Article-13 identifiers and required fields.
    ///
    /// Call before sending to the EU registry to catch structural errors
    /// (GTIN checksum, invalid country codes) before a network round-trip.
    pub fn validate(&self) -> Result<(), RegistryValidationError> {
        self.product_id.validate()?;
        self.level.validate()?;
        match (&self.item_id, self.level.granularity) {
            // Art. 8(1): an item-level registration identifies the unit it covers.
            (None, Granularity::Item) => {
                return Err(RegistryValidationError::MissingRequiredField(
                    "itemId".into(),
                ));
            }
            // Above item level the registration covers a group, so an item
            // identifier contradicts the level the registry validates.
            (Some(_), granularity @ (Granularity::Model | Granularity::Batch)) => {
                return Err(RegistryValidationError::GranularityMismatch {
                    granularity: granularity.wire_str(),
                    identifier: "itemId",
                });
            }
            (Some(item_id), Granularity::Item) => item_id.validate()?,
            (None, Granularity::Model | Granularity::Batch) => {}
        }
        self.facility_id.validate()?;
        self.operator_id.validate()?;
        // Structural only: 6/8/10 digits. Whether the code is the *right* one
        // for this product group is the registry's check, against ranges we do
        // not hold. Absent is lawful ("where relevant"); malformed is not.
        if let Some(code) = &self.commodity_code
            && dpp_domain::CommodityCode::parse(code).is_err()
        {
            return Err(RegistryValidationError::InvalidCommodityCode {
                value: code.clone(),
            });
        }
        // A back-up the registry cannot fetch is worse than none declared.
        if let Some(url) = &self.backup_url
            && !url.starts_with("https://")
        {
            return Err(RegistryValidationError::InsecureBackupUrl { value: url.clone() });
        }
        for (name, value) in [
            ("productGroup", &self.product_group),
            ("schemaVersion", &self.schema_version),
            ("digitalLinkUrl", &self.digital_link_url),
        ] {
            if value.is_empty() {
                return Err(RegistryValidationError::MissingRequiredField(name.into()));
            }
        }
        Ok(())
    }
}

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
