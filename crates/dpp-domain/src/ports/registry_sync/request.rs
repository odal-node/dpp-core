//! [`RegistrationRequest`] — what is sent to the EU Central Registry.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::granularity::RegistrationGranularity;
use super::registering_operator::RegisteringOperator;
use crate::passport::PassportId;

/// Registration request sent to the EU Central Registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrationRequest {
    /// Idempotency key for this registration, minted **once** when the
    /// registration is first built and replayed unchanged on every retry.
    ///
    /// Delivery is at-least-once: a submission the registry commits but whose
    /// response is lost will be retried. A key minted per attempt would make
    /// each retry look like a fresh registration; carried on the request, it is
    /// frozen into the queued payload and survives restarts.
    #[serde(default = "uuid::Uuid::now_v7")]
    pub request_id: uuid::Uuid,
    /// The DPP passport ID (internal to our system).
    pub passport_id: PassportId,
    /// Economic operator's DID or EU-assigned identifier.
    pub operator_identifier: String,
    /// The scheme [`Self::operator_identifier`] is expressed in — `"vat"`,
    /// `"lei"`, `"eori"`, `"duns"`, `"did"`, …
    ///
    /// Carried explicitly because the passport stamps only the identifier's
    /// *value*, and a value alone does not say what it is. Submitting a VAT
    /// number under the wrong scheme is a false statement to the registry that
    /// no structural check can catch — `"did"` in particular is accepted without
    /// verification, so a mis-scheme there is silent.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub operator_identifier_scheme: String,
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
    pub facility: Option<crate::passport::FacilitySnapshot>,
    /// Product category for product group routing within the registry.
    pub product_category: String,
    /// GS1 Digital Link URI or DID URI resolving to the DPP data.
    pub data_carrier_uri: String,
    /// The schema version used for this passport's product group data.
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
    /// Customs tariff classification, copied from the passport.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commodity_code: Option<String>,
    /// Public URL of the passport's back-up, hosted independently of the live
    /// node, where the deployment maintains one.
    ///
    /// The registry verifies "the link to the back-up hosted by a digital
    /// product passport service provider" as part of registration. `None` when
    /// no back-up is *published* — storing snapshots is not the same as serving
    /// them, and declaring a URL nobody can fetch would be worse than declaring
    /// none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backup_url: Option<String>,
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
        passport: &crate::passport::Passport,
        operator: RegisteringOperator<'_>,
        granularity: RegistrationGranularity,
    ) -> Self {
        let product_category = passport.product_group.wire_str().to_owned();
        Self {
            // Minted here, at the one moment a registration comes into
            // existence, and never again — see the field's docs.
            request_id: uuid::Uuid::now_v7(),
            passport_id: passport.id,
            operator_identifier: passport.operator_identifier.clone().unwrap_or_default(),
            operator_identifier_scheme: operator.identifier_scheme.to_owned(),
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
            // Linked where the product group carries one — Art. 8(4)/(5). An
            // absent model identifier is the lawful "no model design exists",
            // so it is read from the product group data rather than assumed.
            model_id: passport
                .product_group_data
                .as_ref()
                .and_then(|d| d.model_identifier())
                .map(ToOwned::to_owned),
            commodity_code: passport.commodity_code.as_ref().map(ToString::to_string),
            // Set by the caller: whether a published back-up exists is a
            // deployment fact, not something the passport records.
            backup_url: None,
        }
    }
}
