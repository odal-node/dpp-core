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
    /// The passport's unique product identifier, in whichever EN 18219 clause 5
    /// scheme issued it.
    ///
    /// 🚨 **Without this an adapter has to invent one.** The registry payload
    /// requires a product identifier, and before this field the only thing on
    /// the request resembling one was
    /// [`data_carrier_uri`](Self::data_carrier_uri) — so a consumer scraped the
    /// GTIN out of it and, when there was none, fell back to the internal
    /// passport UUID. That registers a product with a public authority under a
    /// value meaningless outside the issuing node, and nothing catches it:
    /// `dpp_registry::ProductIdentifier::validate` checks structure only for
    /// `"gtin"`, so an invented scheme passes unexamined.
    ///
    /// Scheme 1 is the case that hid it. While every passport carried a GTIN the
    /// scrape always succeeded; the moment a passport could be identified
    /// without GS1, the fallback became reachable for exactly those passports.
    ///
    /// # Why `Option` when a registration cannot do without it
    ///
    /// The **wire**, not the rule. This type is queued: a consumer's outbox
    /// holds serialised requests across restarts, and a newly required field
    /// makes every already-queued row undeserialisable — a runtime failure
    /// against data, discovered per retry. `Option` lets those rows read.
    ///
    /// The rule lives in
    /// [`from_published_passport`](Self::from_published_passport), which refuses
    /// to build a request without one. A request that reaches an adapter with
    /// `None` came from an older queue, not from this constructor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product_identifier: Option<crate::identifier::ProductIdentifier>,
    /// GS1 Digital Link URI or DID URI resolving to the DPP data.
    ///
    /// The **carrier**, not the identifier — see
    /// [`product_identifier`](Self::product_identifier). A Digital Link happens
    /// to contain a scheme 1 identifier; a scheme 2 or 3 carrier does not, and
    /// reading one out of it is where the invented fallback came from.
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
    /// Who hosts that back-up — ESPR Annex III point (l).
    ///
    /// Separate from the URL because the act lists them separately, and a URL
    /// cannot stand in for a party. Where a `backup_url` is declared this is
    /// not optional in substance: Art. 10(4) says the back-up is made available
    /// *through* a service provider, so one exists whenever a link does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_provider: Option<super::ServiceProviderRef>,
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
    ///
    /// # Errors
    ///
    /// [`ValidationErrors`](crate::field_error::ValidationErrors) naming **every**
    /// field the passport does not carry that a registration cannot do without:
    /// the operator identifier, the facility, and the data carrier URI.
    ///
    /// 🚨 **This used to return `Self`, defaulting all three to `""`.** A
    /// passport published without them produced a request that *looked*
    /// complete — every field populated, nothing optional left unset — and was
    /// refused downstream as `InvalidOperatorId { scheme: "vat", value: "" }`,
    /// which names the scheme it was given and not the fact that nothing was
    /// given.
    ///
    /// Worse, whether it was refused at all depended on the scheme:
    /// `validate_operator_scheme` accepts every unrecognised scheme **without
    /// looking at the value**, so the same empty identifier under `"did"` passed
    /// validation entirely and would have been submitted as though it identified
    /// someone. A registration is the one outbound surface where an empty value
    /// is a statement to a public authority rather than a local mistake.
    ///
    /// The port's own `notify_transfer` documentation
    /// already recorded this shape — *"it could only send empty strings for data
    /// the system had already collected"*. There the adapter had no room; here
    /// the constructor had the room and filled it with nothing.
    ///
    /// The `operator`'s own fields are **not** checked: they are the caller's
    /// input rather than something derived from the passport, so an empty legal
    /// name is a different class of mistake and is caught where the payload is
    /// validated.
    pub fn from_published_passport(
        passport: &crate::passport::Passport,
        operator: RegisteringOperator<'_>,
        granularity: RegistrationGranularity,
    ) -> Result<Self, crate::field_error::ValidationErrors> {
        let product_category = passport.product_group.wire_str().to_owned();

        // 🚨 Every missing field at once, not the first.
        //
        // The opposite of `RegistrationSubmission::validate`, which stops at the
        // first bad passport because the registry's own outcome is one refusal
        // of the whole submission and listing more would misdescribe it. Here
        // nothing has been sent: the caller is fixing their own passport before
        // it travels, and a second round-trip per missing field is a worse
        // answer than a list.
        let mut missing: Vec<crate::field_error::FieldError> = Vec::new();
        let mut require = |present: bool, field: &str, message: &str| {
            if !present {
                missing.push(crate::field_error::FieldError {
                    field: field.to_owned(),
                    message: message.to_owned(),
                });
            }
        };
        // 🚨 Present **and non-blank**. `Some("")` is not a value the passport
        // carried — it is the same absence wearing an `Option::Some`, and
        // `Passport`'s fields are public so nothing stops one being written
        // directly or deserialised from a stored document. Checking `is_some`
        // alone would have left this constructor doing exactly what it was
        // changed to stop doing, one layer in.
        let stated = |value: &str| !value.trim().is_empty();
        require(
            passport.operator_identifier.as_deref().is_some_and(stated),
            "/operatorIdentifier",
            "the registry records the responsible economic operator; a registration \
             cannot name one the passport never carried",
        );
        require(
            passport.facility.as_ref().is_some_and(|f| stated(&f.value)),
            "/facility",
            "Annex III point (i) makes the facility identifier registration data",
        );
        require(
            passport.qr_code_url.as_deref().is_some_and(stated),
            "/qrCodeUrl",
            "the data carrier URI is what the registration resolves to",
        );
        // 🚨 IR (EU) 2026/1778 **Art. 9(2)(a)** is the unconditional hook, not
        // Art. 8. Art. 8 registers a *passport*: 8(8) generates the registry's
        // own *registration* identifier, and 8(9)(a) is a Commission storage
        // duty qualified *"where relevant"* — neither requires the operator to
        // supply a product identifier. Art. 9(2) does: a proof of registration
        // *"shall contain at least … (a) the unique product identifier"*, with
        // no *"where relevant"*, in deliberate contrast to (b) and (c) beside
        // it, which carry one. A registration that could never yield a valid
        // proof is defective at the moment it is built.
        //
        // What must be supplied is also constrained, which is why the internal
        // UUID fallback was never a substitute: ESPR Art. 2(30) defines the
        // identifier as one that *"enables a web link to the digital product
        // passport"*, and Art. 10(c) requires it to comply with the Annex III
        // standards — EN 18219 clause 5. A node-local UUID under an invented
        // `"passport_id"` scheme satisfies neither.
        //
        // `UnsoldGoods` is the known case and is not a defect: an Art. 24–25
        // discard disclosure covers a financial year across many products and
        // identifies no single one, so it has no identifier to give. A
        // disclosure is not a product registration, and refusing it here says
        // so — rather than an `Option` that lets it through unnamed.
        //
        // 🚨 `Other` is refused too, and that case is *not* settled.
        // `product_identifier()` answers `None` for it because the payload is
        // untyped, not because it identifies nothing — an `Other` object can
        // carry a usable `productIdentifier` and is still refused, because
        // nothing reads one out of untyped data. That makes registering a
        // product group added to the catalog after this crate shipped need a
        // release, which is exactly the property `Other` exists to avoid.
        // Fail-closed, so nothing is invented. Tracked in #321.
        let product_identifier = passport
            .product_group_data
            .as_ref()
            .and_then(|d| d.product_identifier())
            .cloned();
        require(
            product_identifier.is_some(),
            "/productGroupData/productIdentifier",
            "Art. 9(2)(a) requires the unique product identifier in a proof of \
             registration; a product group that identifies no single product — \
             an unsold-goods disclosure — is not a product registration",
        );
        if !missing.is_empty() {
            return Err(crate::field_error::ValidationErrors { errors: missing });
        }

        Ok(Self {
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
            product_identifier,
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
            // Same: the provider is a contractual fact the passport does not
            // record, and inventing one would name a party that may not exist.
            service_provider: None,
        })
    }
}
