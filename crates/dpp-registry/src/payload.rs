//! [`RegistrationPayload`] and its [`EuRegistryEnvelope`] wrapper.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::error::RegistryValidationError;
use super::granularity::{Granularity, RegistrationLevel};
use super::identifiers::{
    FacilityIdentifier, OperatorIdentifier, ProductIdentifier, ProductItemIdentifier,
    ServiceProviderReference,
};
use super::submission::{MAX_PRODUCT_IDENTIFIER_CHARS, RegistrationSubmission};

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
    ///
    /// A **location**. The party is
    /// [`service_provider`](Self::service_provider), and the two are not
    /// interchangeable — IR (EU) 2026/1778 lists them in consecutive paragraphs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backup_url: Option<String>,
    /// The digital product passport service provider hosting the back-up —
    /// ESPR Annex III point **(l)**, IR (EU) 2026/1778 Art. 8(9)(c).
    ///
    /// `Option` because Art. 8(9)(c) stores it *"where relevant"* and this crate
    /// cannot tell when it is: a registration whose back-up the operator has not
    /// declared to the registry is not thereby a registration with no provider.
    ///
    /// 🚨 **But a declared [`backup_url`](Self::backup_url) makes it relevant,
    /// and `validate` enforces that.** ESPR Art. 10(4) is unconditional and
    /// names the party: *"The economic operator, when placing the product on the
    /// market, shall make available a back-up copy of the digital product
    /// passport **through a digital product passport service provider**."* So a
    /// URL with no provider beside it describes a back-up that, as a matter of
    /// law, someone is hosting — and Annex III(l) is the point that says who.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_provider: Option<ServiceProviderReference>,
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
        // ESPR Art. 10(4): a back-up copy is made available *through* a service
        // provider, unconditionally. Declaring the link while naming nobody
        // therefore leaves out an Annex III(l) data point that the declaration
        // itself proves exists — so the pair is checked together rather than the
        // provider being validated only when someone remembers to send it.
        //
        // Not the converse. A provider with no link declared is lawful: Art.
        // 8(7)(e) confirms the link "where relevant", and a node may hold the
        // relationship without publishing the URL to the registry.
        match (&self.backup_url, &self.service_provider) {
            (Some(_), None) => {
                return Err(RegistryValidationError::MissingRequiredField(
                    "serviceProvider".into(),
                ));
            }
            (_, Some(provider)) => provider.validate()?,
            (None, None) => {}
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
        // The registry caps the unique product identifier. Counted in `char`s
        // rather than bytes: the guide says "chars", and a host with a non-ASCII
        // internationalised domain would otherwise be measured as longer than
        // what the registry counts.
        let identifier_chars = self.digital_link_url.chars().count();
        if identifier_chars > MAX_PRODUCT_IDENTIFIER_CHARS {
            return Err(RegistryValidationError::ProductIdentifierTooLong {
                chars: identifier_chars,
                max: MAX_PRODUCT_IDENTIFIER_CHARS,
            });
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
    /// **Our** request identifier, for correlating a retry with the attempt it
    /// repeats.
    ///
    /// 🚨 **This is not the registry's idempotency key**, and it used to be
    /// documented as though it were. The registry takes one as an HTTP header —
    /// [`IDEMPOTENCY_KEY_HEADER`](crate::IDEMPOTENCY_KEY_HEADER), observed
    /// 2026-09-16 — so a value carried here reaches it as ordinary payload and
    /// de-duplicates nothing.
    ///
    /// Kept because it is independently useful: minted once and replayed
    /// unchanged, it lets an outbox recognise its own retries without depending
    /// on anything the registry does. An adapter must send the header as well,
    /// and may reasonably send this same value in it.
    pub request_id: Uuid,
    /// ISO 8601 timestamp of when the request was created.
    pub timestamp: DateTime<Utc>,
    /// The passports being registered, which succeed or fail together.
    ///
    /// Was a single `RegistrationPayload`. The registry's unit is the
    /// submission — up to 100 passports, rejected as a whole if any one of them
    /// is bad — so a one-passport envelope was a shape the registry does not
    /// have, and it made both submission caps unenforceable. Use
    /// [`RegistrationSubmission::single`] for the common case.
    pub submission: RegistrationSubmission,
}

impl EuRegistryEnvelope {
    /// Validate the submission this envelope carries.
    ///
    /// # Errors
    ///
    /// Whatever [`RegistrationSubmission::validate`] refuses — a single failing
    /// passport refuses the envelope, naming its index.
    pub fn validate(&self) -> Result<(), RegistryValidationError> {
        self.submission.validate()
    }
}
