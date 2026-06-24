//! Port trait for EU Central DPP Registry synchronisation.
//!
//! ESPR Article 13 mandates a central EU registry that stores at minimum
//! the unique identifiers for every product placed on the market. The registry
//! is scheduled to go live on 19 July 2026.
//!
//! This port defines the interface that platform adapters implement once the
//! Commission publishes the registry API specification. Until then, a no-op
//! `GhostRegistrySync` implementation is provided for testing and development.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::{error::DppError, passport::PassportId};

// ─── Types ───────────────────────────────────────────────────────────────

/// The four persistent identifiers mandated by ESPR Article 13.
///
/// Every product registered in the EU Central Registry receives four
/// identifiers that persist throughout its lifecycle, even across
/// ownership transfers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryIdentifiers {
    /// Unique product identifier within the EU registry.
    pub product_id: String,
    /// Identifier of the economic operator who placed the product on the market.
    pub operator_id: String,
    /// Identifier of the facility where the product was manufactured or imported.
    pub facility_id: String,
    /// The registry's own record identifier.
    pub registry_id: String,
}

/// Registration request sent to the EU Central Registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrationRequest {
    /// The DPP passport ID (internal to our system).
    pub passport_id: PassportId,
    /// Economic operator's DID or EU-assigned identifier.
    pub operator_identifier: String,
    /// Facility identifier (EU-assigned or self-declared).
    pub facility_identifier: String,
    /// Product category for sector routing within the registry.
    pub product_category: String,
    /// GS1 Digital Link URI or DID URI resolving to the DPP data.
    pub data_carrier_uri: String,
    /// The schema version used for this passport's sector data.
    pub schema_version: String,
    /// JWS signature of the DPP payload, for registry integrity binding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jws_signature: Option<String>,
    /// Timestamp when the passport was first published (sourced from the passport, not request time).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub published_at: Option<DateTime<Utc>>,
    /// ISO 3166-1 alpha-2 country code of the responsible operator.
    /// Sourced from `OperatorConfig.country` at publish time.
    /// Empty when operator config has no country set.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub country_code: String,
}

impl RegistrationRequest {
    /// Build a registration request from a published passport.
    ///
    /// All fields are sourced directly from the passport. `country_code` must
    /// be supplied separately (from `OperatorConfig.country`) since the passport
    /// does not store operator country.
    pub fn from_published_passport(
        passport: &crate::domain::passport::Passport,
        country_code: &str,
    ) -> Self {
        let product_category = serde_json::to_value(&passport.sector)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default();
        Self {
            passport_id: passport.id,
            operator_identifier: passport.operator_identifier.clone().unwrap_or_default(),
            facility_identifier: passport.facility_id.clone().unwrap_or_default(),
            product_category,
            data_carrier_uri: passport.qr_code_url.clone().unwrap_or_default(),
            schema_version: passport.schema_version.clone(),
            jws_signature: passport.jws_signature.clone(),
            published_at: passport.published_at,
            country_code: country_code.to_owned(),
        }
    }
}

/// Status of a DPP record within the EU Central Registry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum RegistryStatus {
    /// Registration submitted but not yet confirmed by the registry.
    Pending,
    /// Successfully registered and identifiers assigned.
    Registered,
    /// Registration rejected (e.g. missing fields, invalid operator).
    Rejected,
    /// Record updated after a transfer of responsibility.
    Transferred,
    /// Record suspended by a market surveillance authority.
    SuspendedByAuthority,
}

/// A confirmed registration record returned by the EU Central Registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryRecord {
    /// The four persistent identifiers assigned by the registry.
    pub identifiers: RegistryIdentifiers,
    /// Current status of this registration.
    pub status: RegistryStatus,
    /// Timestamp when the registration was confirmed.
    pub registered_at: DateTime<Utc>,
    /// Timestamp of the most recent status change.
    pub updated_at: DateTime<Utc>,
}

// ─── Port Trait ──────────────────────────────────────────────────────────

/// Port trait for synchronising DPP records with the EU Central Registry.
///
/// The Commission's registry API specification is pending (expected mid-2026).
/// This trait defines the contract that platform adapters will implement.
///
/// # Ghost implementation
///
/// Until the API is published, platform code should wire `GhostRegistrySync`
/// which logs the call and returns a synthetic `RegistryRecord` with
/// `RegistryStatus::Pending`.
#[async_trait]
pub trait RegistrySyncPort: Send + Sync {
    /// Register a new DPP with the EU Central Registry.
    ///
    /// Called when a passport transitions from Draft to Published.
    /// Returns the registry's confirmation record with assigned identifiers.
    async fn register(&self, request: RegistrationRequest) -> Result<RegistryRecord, DppError>;

    /// Query the current status of a previously registered DPP.
    async fn check_status(&self, passport_id: PassportId) -> Result<RegistryRecord, DppError>;

    /// Update a registry record after a transfer of responsibility.
    ///
    /// Called when a product's responsible economic operator changes
    /// (e.g. remanufacturing, repurposing, import into a new market).
    async fn notify_transfer(
        &self,
        passport_id: PassportId,
        new_operator_identifier: String,
    ) -> Result<RegistryRecord, DppError>;
}

// ─── Ghost implementation (development / pre-API) ────────────────────────

/// No-op implementation for use before the EU Central Registry API is published.
///
/// Returns synthetic records with `RegistryStatus::Pending` and placeholder
/// identifiers. All operations succeed but perform no real network calls.
pub struct GhostRegistrySync;

#[async_trait]
impl RegistrySyncPort for GhostRegistrySync {
    async fn register(&self, request: RegistrationRequest) -> Result<RegistryRecord, DppError> {
        let now = Utc::now();
        Ok(RegistryRecord {
            identifiers: RegistryIdentifiers {
                product_id: format!("GHOST-PROD-{}", request.passport_id),
                operator_id: format!("GHOST-OP-{}", &request.operator_identifier),
                facility_id: format!("GHOST-FAC-{}", &request.facility_identifier),
                registry_id: format!("GHOST-REG-{}", Uuid::now_v7()),
            },
            status: RegistryStatus::Pending,
            registered_at: now,
            updated_at: now,
        })
    }

    async fn check_status(&self, passport_id: PassportId) -> Result<RegistryRecord, DppError> {
        Err(DppError::NotFound(format!(
            "ghost registry has no record for {passport_id}"
        )))
    }

    async fn notify_transfer(
        &self,
        passport_id: PassportId,
        _new_operator_identifier: String,
    ) -> Result<RegistryRecord, DppError> {
        Err(DppError::NotFound(format!(
            "ghost registry has no record for {passport_id}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        passport::{ManufacturerInfo, Passport, PassportId},
        sector::Sector,
        status::PassportStatus,
    };
    use chrono::Utc;

    fn make_published_passport() -> Passport {
        Passport {
            id: PassportId::new(),
            batch_id: None,
            product_name: "Test".into(),
            sector: Sector::Textile,
            product_category: None,
            manufacturer: ManufacturerInfo {
                name: "ACME".into(),
                address: "Berlin".into(),
                did_web_url: None,
            },
            materials: vec![],
            co2e_per_unit: None,
            repairability_score: None,
            sector_data: None,
            status: PassportStatus::Published,
            qr_code_url: Some("https://id.odal-node.io/01/09506000134352".into()),
            jws_signature: Some("eyJ0eXAiOiJKV1QifQ.payload.sig".into()),
            public_jws_signature: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            published_at: Some(Utc::now()),
            schema_version: "1.1.0".into(),
            retention_locked: true,
            version: 1,
            supersedes_id: None,
            retention_until: None,
            product_id: None,
            operator_identifier: Some("did:web:acme.example.com".into()),
            facility_id: Some("FAC-DE-001".into()),
        }
    }

    #[tokio::test]
    async fn ghost_register_returns_pending() {
        let sync = GhostRegistrySync;
        let request = RegistrationRequest {
            passport_id: PassportId::new(),
            operator_identifier: "did:web:acme.example.com".into(),
            facility_identifier: "FAC-001".into(),
            product_category: "textile".into(),
            data_carrier_uri: "https://id.example.com/01/09506000134352".into(),
            schema_version: "1.0.0".into(),
            jws_signature: None,
            published_at: None,
            country_code: String::new(),
        };
        let record = sync.register(request).await.unwrap();
        assert_eq!(record.status, RegistryStatus::Pending);
        assert!(record.identifiers.product_id.starts_with("GHOST-PROD-"));
    }

    #[tokio::test]
    async fn ghost_check_status_returns_not_found() {
        let sync = GhostRegistrySync;
        let result = sync.check_status(PassportId::new()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn ghost_notify_transfer_returns_not_found() {
        let sync = GhostRegistrySync;
        let result = sync
            .notify_transfer(PassportId::new(), "did:web:new-operator.example.com".into())
            .await;
        assert!(result.is_err());
    }

    #[test]
    fn from_published_passport_maps_all_fields() {
        let passport = make_published_passport();
        let req = RegistrationRequest::from_published_passport(&passport, "DE");

        assert_eq!(req.passport_id, passport.id);
        assert_eq!(req.operator_identifier, "did:web:acme.example.com");
        assert_eq!(req.facility_identifier, "FAC-DE-001");
        assert_eq!(req.product_category, "textile");
        assert_eq!(
            req.data_carrier_uri,
            "https://id.odal-node.io/01/09506000134352"
        );
        assert_eq!(req.schema_version, "1.1.0");
        assert!(req.jws_signature.is_some());
        assert!(req.published_at.is_some());
        assert_eq!(req.country_code, "DE");
    }

    #[test]
    fn from_published_passport_empty_optionals_produce_empty_strings() {
        let mut passport = make_published_passport();
        passport.operator_identifier = None;
        passport.facility_id = None;
        passport.qr_code_url = None;
        let req = RegistrationRequest::from_published_passport(&passport, "");

        assert!(req.operator_identifier.is_empty());
        assert!(req.facility_identifier.is_empty());
        assert!(req.data_carrier_uri.is_empty());
        assert!(req.country_code.is_empty());
    }

    #[test]
    fn registry_status_serde_round_trip() {
        let statuses = vec![
            RegistryStatus::Pending,
            RegistryStatus::Registered,
            RegistryStatus::Rejected,
            RegistryStatus::Transferred,
            RegistryStatus::SuspendedByAuthority,
        ];
        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            let back: RegistryStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(status, back);
        }
    }
}
