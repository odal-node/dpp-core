//! Serde round-trip and B1 identifier-validation tests for the registry types.

use super::*;
use chrono::Utc;
use uuid::Uuid;

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

pub(crate) fn sample_payload() -> RegistrationPayload {
    RegistrationPayload {
        passport_id: Uuid::nil(),
        product_id: sample_product_id(),
        // Item level: the only level the registry currently accepts, and the
        // one the battery product group is defined at.
        level: RegistrationLevel::new(Granularity::Item).with_model("MODEL-1"),
        item_id: Some(sample_item_id()),
        facility_id: sample_facility_id(),
        operator_id: sample_operator_id(),
        product_group: "textile".into(),
        schema_version: "1.1.0".into(),
        digital_link_url: "https://id.ecotextile.de/01/09506000134352/21/ABC123".into(),
        published_at: Utc::now(),
        jws_signature: Some("eyJhbGciOiJFZERTQSJ9...".into()),
        commodity_code: Some("85076000".into()),
        backup_url: Some("https://backup.example.com/dpp/abc.json".into()),
        service_provider: Some(crate::ServiceProviderReference::named(
            "Example Backup GmbH",
        )),
    }
}

#[test]
fn registration_payload_round_trip() {
    let payload = sample_payload();
    let json = serde_json::to_value(&payload).unwrap();
    assert_eq!(json["productGroup"], "textile");
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
        submission: RegistrationSubmission::single(sample_payload()),
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
        node_acceptance_attestation: Some("sig_to...".into()),
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
    // The Commission's test environment is the `acc` sibling of the production
    // host, not a "sandbox"-named one. This asserted `contains("sandbox")` while
    // the URL was invented, which is precisely the shape of test that agrees
    // with our own guess instead of an outside fact.
    assert_eq!(
        ep.base_url,
        "https://registry.acc.product-passport.ec.europa.eu/api/v1"
    );
}

/// The two environments must never collapse onto one host — submitting test
/// data to the operational registry is not a recoverable mistake.
#[test]
fn sandbox_and_production_are_different_hosts() {
    let sandbox = RegistryEndpoint::sandbox();
    let production = RegistryEndpoint::production();
    assert_ne!(sandbox.base_url, production.base_url);
    assert!(
        sandbox.base_url.contains(".acc."),
        "the test environment is the `acc` host: {}",
        sandbox.base_url
    );
    assert!(
        !production.base_url.contains(".acc."),
        "production must not point at the test environment: {}",
        production.base_url
    );
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
fn empty_country_rejected() {
    // `country` is a mandatory Annex III field — an empty value is a missing
    // required identifier, not an acceptable "unknown".
    let fac = FacilityIdentifier {
        scheme: "national".into(),
        value: "FAC-001".into(),
        name: None,
        country: String::new(),
        address: None,
    };
    assert!(matches!(
        fac.validate(),
        Err(RegistryValidationError::MissingRequiredField(_))
    ));
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
fn empty_item_id_rejected() {
    let mut payload = sample_payload();
    payload.item_id = Some(ProductItemIdentifier {
        scheme: String::new(),
        value: String::new(),
        batch_id: None,
    });
    assert!(matches!(
        payload.validate(),
        Err(RegistryValidationError::MissingRequiredField(_))
    ));
}

#[test]
fn empty_product_group_or_schema_version_rejected() {
    let mut payload = sample_payload();
    payload.product_group = String::new();
    assert!(matches!(
        payload.validate(),
        Err(RegistryValidationError::MissingRequiredField(_))
    ));

    let mut payload = sample_payload();
    payload.schema_version = String::new();
    assert!(matches!(
        payload.validate(),
        Err(RegistryValidationError::MissingRequiredField(_))
    ));
}

#[test]
fn empty_operator_name_rejected() {
    let op = OperatorIdentifier {
        scheme: "vat".into(),
        value: "DE123456789".into(),
        name: String::new(),
        country: "DE".into(),
        did: None,
    };
    assert!(matches!(
        op.validate(),
        Err(RegistryValidationError::MissingRequiredField(_))
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

// ── Registration level (IR (EU) 2026/1778 Art. 8) ───────────────────────────

/// Art. 8(1): an item-level registration must identify the unit it covers.
#[test]
fn item_level_payload_without_an_item_id_rejected() {
    let mut payload = sample_payload();
    payload.item_id = None;
    assert!(matches!(
        payload.validate(),
        Err(RegistryValidationError::MissingRequiredField(f)) if f == "itemId"
    ));
}

/// Above item level the registration covers a group, so naming a single unit
/// contradicts the Art. 8(1) level the registry checks under Art. 8(7)(c).
#[test]
fn model_and_batch_level_payloads_must_not_carry_an_item_id() {
    for granularity in [Granularity::Model, Granularity::Batch] {
        let mut payload = sample_payload();
        payload.level = RegistrationLevel::new(granularity);
        assert!(
            matches!(
                payload.validate(),
                Err(RegistryValidationError::GranularityMismatch {
                    identifier: "itemId",
                    ..
                })
            ),
            "a {granularity} registration must not carry an item identifier"
        );
    }
}

/// A model- or batch-level registration is valid without an item identifier —
/// the levels the registry will accept once further product groups land.
#[test]
fn model_and_batch_level_payloads_validate_without_an_item_id() {
    for granularity in [Granularity::Model, Granularity::Batch] {
        let mut payload = sample_payload();
        payload.level = RegistrationLevel::new(granularity);
        payload.item_id = None;
        assert!(
            payload.validate().is_ok(),
            "a {granularity}-level registration needs no item identifier"
        );
    }
}

/// The level travels on the wire — the registry validates it on submission.
#[test]
fn registration_level_serialises_into_the_payload() {
    let payload = sample_payload();
    let json = serde_json::to_value(&payload).unwrap();
    assert_eq!(json["level"]["granularity"], "item");
    assert_eq!(json["level"]["modelId"], "MODEL-1");
    assert!(
        json["level"].get("batchId").is_none(),
        "an unlinked batch must be absent, not null"
    );
}

#[test]
fn granularity_mismatch_display_message() {
    let err = RegistryValidationError::GranularityMismatch {
        granularity: "model",
        identifier: "batchId",
    };
    assert_eq!(
        err.to_string(),
        "a 'model'-level registration must not carry a 'batchId'"
    );
}

// ── Transfer notification validation ────────────────────────────────────────

fn sample_transfer() -> TransferNotification {
    TransferNotification {
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
        node_acceptance_attestation: Some("sig_to...".into()),
    }
}

#[test]
fn valid_transfer_notification_passes() {
    assert!(sample_transfer().validate().is_ok());
}

/// A transfer names the two legal persons on either side of the handover, so an
/// unidentified operator on *either* side must be refused. This is the check
/// whose absence let an adapter send empty strings for both.
#[test]
fn transfer_with_an_unidentified_operator_is_refused() {
    let mut from_blank = sample_transfer();
    from_blank.from_operator.name = String::new();
    assert!(
        matches!(
            from_blank.validate(),
            Err(RegistryValidationError::MissingRequiredField(f)) if f == "operatorId.name"
        ),
        "the outgoing operator must be identified"
    );

    let mut to_blank = sample_transfer();
    to_blank.to_operator.name = String::new();
    assert!(
        matches!(
            to_blank.validate(),
            Err(RegistryValidationError::MissingRequiredField(f)) if f == "operatorId.name"
        ),
        "the incoming operator must be identified"
    );
}

#[test]
fn transfer_with_an_invalid_country_is_refused() {
    let mut notif = sample_transfer();
    notif.to_operator.country = "XX".into();
    assert!(matches!(
        notif.validate(),
        Err(RegistryValidationError::InvalidCountryCode { .. })
    ));
}

#[test]
fn transfer_without_a_reason_is_refused() {
    let mut notif = sample_transfer();
    notif.reason = "   ".into();
    assert!(matches!(
        notif.validate(),
        Err(RegistryValidationError::MissingRequiredField(f)) if f == "reason"
    ));
}

/// A transfer is initiated by the outgoing operator and countersigned only when
/// the incoming one accepts, so a pending transfer legitimately has no
/// `node_acceptance_attestation`. Requiring it here would make the notification unbuildable
/// for exactly the case a registry most wants to hear about.
#[test]
fn a_pending_transfer_still_validates() {
    let mut notif = sample_transfer();
    notif.node_acceptance_attestation = None;
    assert!(notif.validate().is_ok());
}

/// An identifier with no scheme is not identifiable: the value alone does not
/// say whether it is a VAT number, an LEI or a DID. This is the check that
/// stops an unscheme'd identifier being submitted as if it were well-formed —
/// the per-scheme check accepts any unrecognised scheme, including the empty
/// one, so without this it would pass.
#[test]
fn operator_identifier_without_a_scheme_is_refused() {
    for scheme in ["", "   "] {
        let mut oid = sample_operator_id();
        oid.scheme = scheme.into();
        assert!(
            matches!(
                oid.validate(),
                Err(RegistryValidationError::MissingRequiredField(f)) if f == "operatorId.scheme"
            ),
            "an identifier with scheme {scheme:?} must be refused"
        );
    }
}

// ── Commodity code and back-up link ─────────────────────────────────────────

/// Absent is lawful: the regulation qualifies the commodity code "where
/// relevant", and a product group that does not call for one must still register.
#[test]
fn a_payload_without_a_commodity_code_validates() {
    let mut payload = sample_payload();
    payload.commodity_code = None;
    assert!(payload.validate().is_ok());
}

/// Structurally malformed is not. Whether the code is the *right* one for the
/// product group is the registry's check against ranges we do not hold; whether
/// it is a tariff code at all is checkable here.
#[test]
fn a_malformed_commodity_code_is_refused() {
    for bad in ["8507", "8507 60 00", "notacode", "850760009"] {
        let mut payload = sample_payload();
        payload.commodity_code = Some(bad.into());
        assert!(
            matches!(
                payload.validate(),
                Err(RegistryValidationError::InvalidCommodityCode { .. })
            ),
            "{bad} must be refused"
        );
    }
}

#[test]
fn the_three_tariff_levels_all_validate() {
    for good in ["850760", "85076000", "8507600090"] {
        let mut payload = sample_payload();
        payload.commodity_code = Some(good.into());
        assert!(payload.validate().is_ok(), "{good} must validate");
    }
}

/// A back-up the registry cannot fetch over TLS is worse than none declared.
#[test]
fn an_insecure_backup_url_is_refused() {
    let mut payload = sample_payload();
    payload.backup_url = Some("http://backup.example.com/dpp/abc.json".into());
    assert!(matches!(
        payload.validate(),
        Err(RegistryValidationError::InsecureBackupUrl { .. })
    ));
}

/// Declaring no back-up is lawful — storing snapshots is not the same as
/// publishing them, and a deployment that does not publish one says so.
#[test]
fn a_payload_without_a_backup_url_validates() {
    let mut payload = sample_payload();
    payload.backup_url = None;
    assert!(payload.validate().is_ok());
}

/// Both travel on the wire, and both are omitted rather than nulled when unset.
#[test]
fn commodity_code_and_backup_url_serialise() {
    let payload = sample_payload();
    let json = serde_json::to_value(&payload).unwrap();
    assert_eq!(json["commodityCode"], "85076000");
    assert_eq!(json["backupUrl"], "https://backup.example.com/dpp/abc.json");

    let mut bare = sample_payload();
    bare.commodity_code = None;
    bare.backup_url = None;
    let json = serde_json::to_value(&bare).unwrap();
    assert!(json.get("commodityCode").is_none());
    assert!(json.get("backupUrl").is_none());
}

// ── The observed contract ────────────────────────────────────────────────────
//
// Each of these pins something read from the registry's own published material
// on 2026-09-16 — its web client, or the User Guide v1.02. None of it is a
// specification, so the tests exist to make a later correction *visible* rather
// than to assert conformance.

/// The registration path was `/registrations`, which was invented. The client
/// names a different one, so the old value is not merely unverified — it is
/// wrong, and this is the guard against it drifting back.
#[test]
fn the_registration_path_is_the_observed_one() {
    assert_eq!(REGISTRATION_PATH, "/dpp-registration-requests");
    assert_ne!(REGISTRATION_PATH, "/registrations");
}

/// Idempotency is a header, not an envelope field. Both halves matter: the
/// header must be spelled as the registry spells it, and `request_id` must not
/// be mistaken for it again.
#[test]
fn idempotency_is_a_header_and_request_id_is_not_it() {
    assert_eq!(IDEMPOTENCY_KEY_HEADER, "Idempotency-Key");

    let envelope = EuRegistryEnvelope {
        api_version: "1.0".into(),
        request_id: Uuid::now_v7(),
        timestamp: Utc::now(),
        submission: RegistrationSubmission::single(sample_payload()),
    };
    let json = serde_json::to_value(&envelope).unwrap();
    // It travels as ordinary payload under our own name, and de-duplicates
    // nothing at the registry.
    assert!(json.get("requestId").is_some());
    assert!(json.get("idempotencyKey").is_none());
}

/// The 409 the web client recognises. A caller seeing it should read the
/// original submission's outcome rather than resubmit, so recognising it has to
/// be reliable.
#[test]
fn a_replayed_idempotency_key_is_recognisable() {
    let body: RegistryErrorBody = serde_json::from_value(serde_json::json!({
        "subCode": "CONFLICT_IDEMPOTENCY_KEY_ALREADY_USED",
        "traceId": "abc123",
    }))
    .unwrap();

    assert!(body.is_idempotency_key_reused());
    assert_eq!(body.trace_id.as_deref(), Some("abc123"));

    let other: RegistryErrorBody = serde_json::from_value(serde_json::json!({
        "subCode": "SOMETHING_ELSE",
    }))
    .unwrap();
    assert!(!other.is_idempotency_key_reused());

    // An error body carrying neither field is still readable — the shape beyond
    // these two is unknown, so absence must not be a parse failure.
    let bare: RegistryErrorBody = serde_json::from_value(serde_json::json!({})).unwrap();
    assert!(!bare.is_idempotency_key_reused());
    assert!(bare.trace_id.is_none());
}

/// The number that reversed a design risk. A Digital Link of the shape this
/// workspace builds must fit with room to spare — that is the whole point of
/// the constant, and a regression to 50 would fail here rather than at the
/// registry.
#[test]
fn a_digital_link_carrier_url_fits_the_identifier_limit() {
    assert_eq!(MAX_PRODUCT_IDENTIFIER_CHARS, 2000);

    let payload = sample_payload();
    // The real shape: host + GTIN-14 + a 20-character serial. The exact count
    // moves with the host, so what is asserted is the order of magnitude and
    // the headroom — not a figure that a longer hostname would falsify.
    let carrier = "https://id.example.com/01/09506000134352/21/ABCDEFGHIJKLMNOPQRST";
    assert_eq!(carrier.chars().count(), 64);
    assert!(carrier.chars().count() * 30 < MAX_PRODUCT_IDENTIFIER_CHARS);

    let mut fits = payload.clone();
    fits.digital_link_url = carrier.into();
    assert!(fits.validate().is_ok());
}

#[test]
fn an_over_long_product_identifier_is_refused() {
    let mut payload = sample_payload();
    payload.digital_link_url = format!(
        "https://id.example.com/01/{}",
        "0".repeat(MAX_PRODUCT_IDENTIFIER_CHARS)
    );
    let chars = payload.digital_link_url.chars().count();

    assert_eq!(
        payload.validate(),
        Err(RegistryValidationError::ProductIdentifierTooLong {
            chars,
            max: MAX_PRODUCT_IDENTIFIER_CHARS,
        })
    );
}

/// Counted in characters, not bytes. An internationalised host is not longer
/// than the registry thinks it is.
#[test]
fn the_identifier_limit_counts_characters_not_bytes() {
    let mut payload = sample_payload();
    // 1999 characters, of which many are multi-byte — over the limit in bytes,
    // under it in chars.
    payload.digital_link_url = format!("https://é.example/{}", "é".repeat(1981));
    assert_eq!(payload.digital_link_url.chars().count(), 1999);
    assert!(payload.digital_link_url.len() > MAX_PRODUCT_IDENTIFIER_CHARS);
    assert!(payload.validate().is_ok());
}

/// The submission limits, recorded so a batch implementation inherits them
/// rather than rediscovering them at the registry.
#[test]
fn the_observed_submission_limits_are_recorded() {
    assert_eq!(MAX_PASSPORTS_PER_SUBMISSION, 100);
    assert_eq!(MAX_SUBMISSION_BYTES, 1_073_741_824);
}

// ── Evidence tier ────────────────────────────────────────────────────────────

/// 🚨 Every wire constant this crate declares must state what backs it.
///
/// The table is only a contract if nothing can be added beside it. A path
/// declared in `endpoint` and left out of `ENDPOINT_BASIS` is a route with no
/// provenance — which is what every route was before the table existed, and the
/// state a reader cannot distinguish from "we checked and it is fine".
///
/// Listed here by name rather than derived, deliberately: deriving the list from
/// the table would make the table agree with itself.
#[test]
fn every_wire_constant_declares_a_basis() {
    use crate::endpoint::{
        IDEMPOTENCY_KEY_HEADER, REGISTRATION_PATH, STATUS_PATH_TEMPLATE, TRANSFER_PATH_TEMPLATE,
        basis_of,
    };

    for declared in [
        REGISTRATION_PATH,
        STATUS_PATH_TEMPLATE,
        TRANSFER_PATH_TEMPLATE,
        IDEMPOTENCY_KEY_HEADER,
    ] {
        assert!(
            basis_of(declared).is_some(),
            "{declared} is declared by this crate and states no basis — add it to ENDPOINT_BASIS"
        );
    }

    // And nothing in the table that the crate does not declare, so a renamed
    // constant cannot leave a stale entry behind vouching for a dead route.
    assert_eq!(
        crate::ENDPOINT_BASIS.len(),
        4,
        "the table grew or shrank without this test being told"
    );
}

/// What is observed is observed, and what is not says so.
///
/// Pinned per constant rather than counted. A count would still pass if an
/// invented route were quietly relabelled as observed, which is the single
/// change this whole tier exists to make impossible to do silently.
#[test]
fn each_route_carries_the_tier_its_prose_claims() {
    use crate::endpoint::{
        IDEMPOTENCY_KEY_HEADER, REGISTRATION_PATH, STATUS_PATH_TEMPLATE, TRANSFER_PATH_TEMPLATE,
        basis_of,
    };

    // Read off the registry's own web client.
    for observed in [REGISTRATION_PATH, IDEMPOTENCY_KEY_HEADER] {
        let basis = basis_of(observed).expect("declared");
        assert!(basis.is_observed(), "{observed} is observed");
        assert_eq!(
            basis.observed_on(),
            Some("2026-09-16"),
            "an observation must carry the date it was made"
        );
    }

    // Nothing behind them. The asynchronous flow needs *a* status route and
    // transfers need *a* route, and naming one is not knowing it.
    for assumed in [STATUS_PATH_TEMPLATE, TRANSFER_PATH_TEMPLATE] {
        let basis = basis_of(assumed).expect("declared");
        assert!(!basis.is_observed(), "{assumed} rests on nothing");
        assert_eq!(basis.observed_on(), None);
    }
}

/// A value this crate never declared answers `None`, not `Assumed`.
///
/// The difference matters to the caller: `Assumed` says "ours, and unbacked",
/// which is a statement about a route we named. `None` says "we never said
/// this", which for a caller that believes it copied the route from here means
/// a typo or a stale pin — and answering `Assumed` would dress that up as an
/// expected outcome.
#[test]
fn an_undeclared_route_has_no_basis_rather_than_a_default_one() {
    assert_eq!(crate::endpoint::basis_of("/registrations"), None);
    assert_eq!(crate::endpoint::basis_of(""), None);
}

/// Every error kind decides its status here, or says it has none — and every
/// one of those decisions is ours.
///
/// The exhaustive `match` inside `http_status` is what makes a new kind a
/// compile error rather than a silent default; this is the other half, asserting
/// that none of the decisions has quietly been dressed up as observed. The only
/// status anyone has seen is the replayed-key 409.
#[test]
fn every_assumed_status_says_it_is_assumed() {
    use crate::EuRegistryErrorKind as Kind;

    let with_status = [
        (Kind::Unauthorized, 401),
        (Kind::RegistrationRejected, 422),
        (Kind::RateLimited, 429),
        (Kind::NotFound, 404),
        (Kind::RegistryInternalError, 500),
    ];
    for (kind, expected) in with_status {
        let (status, basis) = kind.http_status().expect("this kind is a response");
        assert_eq!(status, expected, "{kind:?}");
        assert!(
            !basis.is_observed(),
            "{kind:?} claims an observed status; only the replayed-key 409 is observed"
        );
    }

    // No response, so no status to report. Inventing one would put a number on
    // the wire that no registry chose.
    for kind in [Kind::ConnectionFailed, Kind::Timeout, Kind::InvalidResponse] {
        assert_eq!(kind.http_status(), None, "{kind:?} describes no response");
    }
}

/// The one status with something behind it.
#[test]
fn the_replayed_key_status_is_the_only_observed_one() {
    assert_eq!(crate::STATUS_IDEMPOTENCY_KEY_REUSED, 409);
    assert!(crate::STATUS_IDEMPOTENCY_KEY_REUSED_BASIS.is_observed());
    assert_eq!(
        crate::STATUS_IDEMPOTENCY_KEY_REUSED_BASIS.observed_on(),
        Some("2026-09-16")
    );
}

// ── Annex III(l): the service provider reference ─────────────────────────────

/// 🚨 A declared back-up link with nobody named is refused.
///
/// ESPR Art. 10(4) is unconditional — a back-up copy is made available *through*
/// a digital product passport service provider — so a payload that declares the
/// link has, as a matter of law, a provider to reference. Annex III(l) is the
/// point that says who, and IR (EU) 2026/1778 Art. 8(9)(c) is the registration
/// data the Commission stores. Sending the location and omitting the party drops
/// a data point the declaration itself proves exists.
#[test]
fn a_backup_url_without_a_provider_is_refused() {
    let mut payload = sample_payload();
    payload.backup_url = Some("https://backup.example.com/dpp/abc.json".into());
    payload.service_provider = None;

    assert!(
        matches!(
            payload.validate(),
            Err(RegistryValidationError::MissingRequiredField(ref f)) if f == "serviceProvider"
        ),
        "a back-up link with no provider named was accepted"
    );
}

/// And not the converse. Art. 8(7)(e) confirms the link *"where relevant"*, so a
/// node may hold the provider relationship without publishing the URL to the
/// registry — which is also the case where a back-up exists and the deployment
/// simply has not declared it here.
#[test]
fn a_provider_without_a_backup_url_is_accepted() {
    let mut payload = sample_payload();
    payload.backup_url = None;
    payload.service_provider = Some(ServiceProviderReference::named("Example Backup GmbH"));

    assert!(payload.validate().is_ok(), "{:?}", payload.validate());
}

/// Neither is a payload that declares no back-up at all.
#[test]
fn declaring_no_backup_needs_no_provider() {
    let mut payload = sample_payload();
    payload.backup_url = None;
    payload.service_provider = None;

    assert!(payload.validate().is_ok(), "{:?}", payload.validate());
}

/// A name is the floor, and it is a real floor: Annex III(l) mandates no
/// identifier scheme for point (l), so a named provider with nothing else is
/// lawful rather than a degraded reference.
#[test]
fn a_name_alone_is_a_complete_reference() {
    assert!(
        ServiceProviderReference::named("Example Backup GmbH")
            .validate()
            .is_ok()
    );
    assert!(matches!(
        ServiceProviderReference::named("   ").validate(),
        Err(RegistryValidationError::MissingRequiredField(ref f)) if f == "serviceProvider.name"
    ));
}

/// 🚨 A scheme and a value arrive together or not at all.
///
/// The reason `OperatorIdentifier::validate` already gives: a value with no
/// scheme does not say whether it is a VAT number, an LEI or a DID, so it
/// identifies nobody while looking as though it does. A scheme with no value is
/// the same defect mirrored — and both would otherwise pass, because the
/// per-scheme check accepts any unrecognised scheme including the empty one.
#[test]
fn a_half_stated_identifier_is_refused() {
    let named = |scheme: Option<&str>, value: Option<&str>| ServiceProviderReference {
        name: "Example Backup GmbH".into(),
        scheme: scheme.map(Into::into),
        value: value.map(Into::into),
        country: None,
    };

    for (scheme, value, expected) in [
        (Some("lei"), None, "serviceProvider.value"),
        (None, Some("529900T8BM49AURSDO55"), "serviceProvider.scheme"),
        (Some("  "), Some("anything"), "serviceProvider.scheme"),
        // 🚨 A blank value under a scheme nothing structurally checks.
        // `validate_operator_scheme` accepts `"did"` and every unrecognised
        // scheme without looking at the value, so these reached `Ok` — and
        // `Some("did")` with an empty value is the same absence as
        // `Some("did")` with no value, which the first case above refuses.
        // Two answers for one state is the defect, not the leniency.
        (Some("did"), Some(""), "serviceProvider.value"),
        (Some("did"), Some("   "), "serviceProvider.value"),
        (Some("something-new"), Some(""), "serviceProvider.value"),
    ] {
        assert!(
            matches!(
                named(scheme, value).validate(),
                Err(RegistryValidationError::MissingRequiredField(ref f)) if f == expected
            ),
            "{scheme:?}/{value:?} should have been refused as {expected}"
        );
    }

    // Stated in full, it goes through the same per-scheme check an operator
    // identifier does — a provider is a legal person identified the same way.
    assert!(
        named(Some("lei"), Some("529900T8BM49AURSDO55"))
            .validate()
            .is_ok()
    );
    assert!(named(Some("lei"), Some("not-an-lei")).validate().is_err());
}

/// The country is validated when present and optional when not — Annex III(l)
/// asks for a reference, not an establishment record.
#[test]
fn a_provider_country_is_optional_and_checked_when_given() {
    let with_country = |country: &str| ServiceProviderReference {
        country: Some(country.into()),
        ..ServiceProviderReference::named("Example Backup GmbH")
    };

    assert!(with_country("DE").validate().is_ok());
    assert!(matches!(
        with_country("Germany").validate(),
        Err(RegistryValidationError::InvalidCountryCode { .. })
    ));
}

// ── EN 18219 clause 5 → registry scheme ──────────────────────────────────────

/// Each clause 5 scheme converts to its own scheme value, and carries the
/// identifier verbatim.
///
/// 🚨 The GS1 arm yields the **bare 14-digit GTIN**, not a Digital Link URL.
/// Building the URL needs a resolver host, which this tier does not know — and
/// a conversion that guessed one would put a resolver nobody configured into a
/// registration.
#[test]
fn each_clause_5_scheme_converts_to_its_own_registry_scheme() {
    use dpp_domain::identifier::ProductIdentifier as Clause5;

    let cases = [
        (
            Clause5::gs1(dpp_domain::Gtin::parse("09506000134352").unwrap()),
            SCHEME_GTIN,
            "09506000134352",
        ),
        (
            Clause5::identification_link("https://id.example.com/p/1").unwrap(),
            SCHEME_IDENTIFICATION_LINK,
            "https://id.example.com/p/1",
        ),
        (
            Clause5::did("did:web:example.com:p:1").unwrap(),
            SCHEME_DID,
            "did:web:example.com:p:1",
        ),
    ];

    for (clause5, expected_scheme, expected_value) in cases {
        let converted = ProductIdentifier::try_from(&clause5).expect("a mapped scheme");
        assert_eq!(converted.scheme, expected_scheme);
        assert_eq!(converted.value, expected_value);
        assert_eq!(converted.label, None);
    }
}

/// The scheme values must be distinct, or two schemes register as one.
///
/// Worth its own assertion because the failure is invisible: a copy-paste
/// leaving two constants equal produces conversions that validate, serialise and
/// round-trip, while filing a DID and a link under one name.
#[test]
fn no_two_schemes_share_a_registry_value() {
    let mut values: Vec<&str> = PRODUCT_SCHEME_BASIS.iter().map(|(v, _)| *v).collect();
    let count = values.len();
    values.sort_unstable();
    values.dedup();
    assert_eq!(
        values.len(),
        count,
        "two clause 5 schemes share a scheme value"
    );
}

/// 🚨 None of the three is observed, and the table says so where code can read it.
///
/// `"gtin"` and `"did"` have in-crate precedent, which is a convention of ours
/// and not evidence about the registry; scheme 2's has none at all. If one is
/// ever read off a real response this fails, which is the point — the upgrade
/// should be deliberate.
#[test]
fn every_scheme_value_is_assumed_not_observed() {
    for (value, basis) in PRODUCT_SCHEME_BASIS {
        assert!(
            !basis.is_observed(),
            "{value} claims to be observed; no registry response has been seen naming any scheme"
        );
    }
}

/// A converted scheme 1 identifier still passes the mod-10 check the registry
/// crate applies to `"gtin"` — the one scheme value with behaviour attached.
#[test]
fn a_converted_gtin_still_validates_as_one() {
    use dpp_domain::identifier::ProductIdentifier as Clause5;

    let good = Clause5::gs1(dpp_domain::Gtin::parse("09506000134352").unwrap());
    let converted = ProductIdentifier::try_from(&good).unwrap();
    assert!(converted.validate().is_ok());

    // And the check really is keyed on the scheme string: relabel a DID as a
    // GTIN and the same validator refuses it, which is why the conversion must
    // not be left to each consumer to spell.
    let did = Clause5::did("did:web:example.com:p:1").unwrap();
    let mut mislabelled = ProductIdentifier::try_from(&did).unwrap();
    mislabelled.scheme = SCHEME_GTIN.to_owned();
    assert!(matches!(
        mislabelled.validate(),
        Err(RegistryValidationError::InvalidGtin { .. })
    ));
}
