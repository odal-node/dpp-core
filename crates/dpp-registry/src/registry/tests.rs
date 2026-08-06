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

fn sample_payload() -> RegistrationPayload {
    RegistrationPayload {
        passport_id: Uuid::nil(),
        product_id: sample_product_id(),
        // Item level: the only level the registry currently accepts, and the
        // one the battery product group is defined at.
        level: RegistrationLevel::new(Granularity::Item).with_model("MODEL-1"),
        item_id: Some(sample_item_id()),
        facility_id: sample_facility_id(),
        operator_id: sample_operator_id(),
        sector: "textile".into(),
        schema_version: "1.1.0".into(),
        digital_link_url: "https://id.ecotextile.de/01/09506000134352/21/ABC123".into(),
        published_at: Utc::now(),
        jws_signature: Some("eyJhbGciOiJFZERTQSJ9...".into()),
        commodity_code: Some("85076000".into()),
        backup_url: Some("https://backup.example.com/dpp/abc.json".into()),
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
fn empty_sector_or_schema_version_rejected() {
    let mut payload = sample_payload();
    payload.sector = String::new();
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
        to_signature: Some("sig_to...".into()),
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
/// `to_signature`. Requiring it here would make the notification unbuildable
/// for exactly the case a registry most wants to hear about.
#[test]
fn a_pending_transfer_still_validates() {
    let mut notif = sample_transfer();
    notif.to_signature = None;
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
