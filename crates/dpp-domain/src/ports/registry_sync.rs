//! Port trait for EU Central DPP Registry synchronisation.
//!
//! ESPR Article 13 establishes a central EU registry that stores at minimum
//! the unique identifiers for every product placed on the market. The registry
//! is scheduled to go live on 19 July 2026.
//!
//! This port defines the interface that platform adapters implement once the
//! Commission publishes the registry API specification. Until then, a no-op
//! `GhostRegistrySync` implementation is provided for testing and development.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::{error::DppError, passport::PassportId};

// ─── Types ───────────────────────────────────────────────────────────────

/// The persistent identifiers a registration carries. Specified by ESPR
/// **Annex III** (product (b), operator (g)/(h), facility (i)); Art. 13 is the
/// registry that stores them, not their definition.
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
    /// Legal name of the responsible economic operator.
    ///
    /// Sourced from the operator's own configuration, not from the passport —
    /// the passport records the *manufacturer*, which is frequently a different
    /// legal person from the operator placing the product on the EU market.
    /// The registry requires a legal-entity name on the operator identifier, so
    /// a registration without one cannot be built.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub operator_name: String,
    /// Facility identifier value (EU-assigned or self-declared) — the flat
    /// convenience form of [`Self::facility`]`.value`, kept for registries/clients
    /// that only consume the bare identifier.
    pub facility_identifier: String,
    /// Full Annex III facility descriptor (scheme, value, name, country, address)
    /// snapshotted onto the passport, so the registry payload can carry the
    /// facility's name/country/scheme rather than a bare identifier. `None` when
    /// the passport was published without a facility.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub facility: Option<crate::domain::passport::FacilitySnapshot>,
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
    /// The model / batch / item level this passport is registered at.
    ///
    /// Set by the applicable delegated act for the product group, not by the
    /// passport — which is why it is supplied by the caller rather than
    /// derived here. Defaults to item level, the only level the registry
    /// currently accepts.
    #[serde(default)]
    pub granularity: RegistrationGranularity,
    /// Identifier of the model this product belongs to, where a model design
    /// exists for it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
}

/// The level a passport is registered at, mirrored in the domain so the port
/// does not depend on the registry wire crate.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RegistrationGranularity {
    /// One registration covering every item sharing a product's specifications.
    Model,
    /// One registration covering every item made in one production run.
    Batch,
    /// One registration per physical unit.
    #[default]
    Item,
}

/// The registering operator's own details, which the passport does not carry.
///
/// A struct rather than loose arguments because `legal_name` and `country` are
/// both plain strings: passed positionally they can be swapped without the
/// compiler noticing, and the result is a registration filed under the wrong
/// legal entity.
#[derive(Debug, Clone, Copy)]
pub struct RegisteringOperator<'a> {
    /// Legal name of the economic operator (`OperatorConfig.legal_name`).
    pub legal_name: &'a str,
    /// ISO 3166-1 alpha-2 country of registration (`OperatorConfig.country`).
    pub country: &'a str,
}

impl RegistrationRequest {
    /// Build a registration request from a published passport.
    ///
    /// Product fields are sourced from the passport. The operator's legal name
    /// and country come from `operator` — the passport records the manufacturer,
    /// which is frequently not the operator placing the product on the market.
    ///
    /// `granularity` is set by the applicable delegated act for the product
    /// group; `model_id` is left unset here and linked by the caller where a
    /// model design exists for the product.
    pub fn from_published_passport(
        passport: &crate::domain::passport::Passport,
        operator: RegisteringOperator<'_>,
        granularity: RegistrationGranularity,
    ) -> Self {
        let product_category = passport.sector.wire_str().to_owned();
        Self {
            passport_id: passport.id,
            operator_identifier: passport.operator_identifier.clone().unwrap_or_default(),
            operator_name: operator.legal_name.to_owned(),
            facility_identifier: passport
                .facility
                .as_ref()
                .map(|f| f.value.clone())
                .unwrap_or_default(),
            facility: passport.facility.clone(),
            product_category,
            data_carrier_uri: passport.qr_code_url.clone().unwrap_or_default(),
            schema_version: passport.schema_version.clone(),
            jws_signature: passport.jws_signature.clone(),
            published_at: passport.published_at,
            country_code: operator.country.to_owned(),
            granularity,
            model_id: None,
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
    ///
    /// Takes the whole [`TransferRecord`](crate::domain::transfer::TransferRecord)
    /// rather than just the incoming
    /// operator's identifier. A registry notification names **both** legal
    /// persons and carries the dual signatures that authorise the handover;
    /// passing only the new identifier left an adapter no way to express the
    /// outgoing operator or either signature, so it could only send empty
    /// strings for data the system had already collected.
    async fn notify_transfer(
        &self,
        record: &crate::domain::transfer::TransferRecord,
    ) -> Result<RegistryRecord, DppError>;
}

// ─── Ghost implementation (development / pre-API) ────────────────────────

/// No-op implementation for use before the EU Central Registry API is published.
///
/// Returns synthetic records with `RegistryStatus::Pending` and placeholder
/// identifiers. All operations succeed but perform no real network calls.
pub use crate::ports::ghosts::GhostRegistrySync;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        passport::{ManufacturerInfo, Passport},
        status::PassportStatus,
    };
    use chrono::Utc;

    fn make_published_passport() -> Passport {
        Passport {
            product_name: "Test".into(),
            manufacturer: ManufacturerInfo {
                name: "ACME".into(),
                address: "Berlin".into(),
                did_web_url: None,
            },
            status: PassportStatus::Published,
            qr_code_url: Some("https://id.odal-node.io/01/09506000134352".into()),
            jws_signature: Some("eyJ0eXAiOiJKV1QifQ.payload.sig".into()),
            published_at: Some(Utc::now()),
            schema_version: "1.1.0".into(),
            retention_locked: true,
            operator_identifier: Some("did:web:acme.example.com".into()),
            facility: Some(crate::domain::passport::FacilitySnapshot {
                scheme: "national".into(),
                value: "FAC-DE-001".into(),
                name: "Acme Plant".into(),
                country: "DE".into(),
                address: None,
            }),
            ..crate::test_support::sample_passport()
        }
    }

    /// The operator identity a test registration is filed under.
    fn acme() -> RegisteringOperator<'static> {
        RegisteringOperator {
            legal_name: "Acme GmbH",
            country: "DE",
        }
    }

    #[test]
    fn from_published_passport_maps_all_fields() {
        let passport = make_published_passport();
        let req = RegistrationRequest::from_published_passport(
            &passport,
            acme(),
            RegistrationGranularity::Item,
        );

        assert_eq!(req.passport_id, passport.id);
        assert_eq!(req.operator_identifier, "did:web:acme.example.com");
        assert_eq!(req.facility_identifier, "FAC-DE-001");
        // The full facility descriptor is carried, not just the bare identifier.
        assert_eq!(
            req.facility.as_ref().map(|f| f.name.as_str()),
            Some("Acme Plant")
        );
        assert_eq!(
            req.facility.as_ref().map(|f| f.country.as_str()),
            Some("DE")
        );
        assert_eq!(req.product_category, "textile");
        assert_eq!(
            req.data_carrier_uri,
            "https://id.odal-node.io/01/09506000134352"
        );
        assert_eq!(req.schema_version, "1.1.0");
        assert!(req.jws_signature.is_some());
        assert!(req.published_at.is_some());
        assert_eq!(req.country_code, "DE");
        // The operator's legal name comes from operator config, never from the
        // passport's manufacturer block.
        assert_eq!(req.operator_name, "Acme GmbH");
        assert_ne!(
            req.operator_name, passport.manufacturer.name,
            "operator and manufacturer are distinct legal persons"
        );
        assert_eq!(req.granularity, RegistrationGranularity::Item);
    }

    #[test]
    fn from_published_passport_empty_optionals_produce_empty_strings() {
        let mut passport = make_published_passport();
        passport.operator_identifier = None;
        passport.facility = None;
        passport.qr_code_url = None;
        let req = RegistrationRequest::from_published_passport(
            &passport,
            RegisteringOperator {
                legal_name: "",
                country: "",
            },
            RegistrationGranularity::Item,
        );

        assert!(req.operator_identifier.is_empty());
        assert!(req.facility_identifier.is_empty());
        assert!(req.facility.is_none());
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
