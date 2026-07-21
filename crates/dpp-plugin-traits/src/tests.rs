//! ABI version negotiation, capability declaration, and wire-envelope tests.

use super::*;

fn sample_capabilities() -> PluginCapabilities {
    PluginCapabilities {
        abi_version: AbiVersion::current(),
        supported_schemas: vec![SchemaVersionRange {
            min_version: "1.0.0".into(),
            max_version: "1.1.0".into(),
        }],
        capabilities: vec![
            PluginCapability::Validate,
            PluginCapability::ComputeMetrics,
            PluginCapability::GeneratePassport,
        ],
        min_host_version: None,
        max_fuel: None,
        max_memory_bytes: None,
    }
}

#[test]
fn abi_version_current_is_compatible() {
    let current = AbiVersion::current();
    assert!(current.is_compatible_with_host());
}

#[test]
fn abi_version_major_mismatch_incompatible() {
    let future = AbiVersion { major: 2, minor: 0 };
    assert!(!future.is_compatible_with_host());
}

#[test]
fn abi_version_minor_ahead_incompatible() {
    let ahead = AbiVersion {
        major: ABI_VERSION_MAJOR,
        minor: ABI_VERSION_MINOR + 1,
    };
    assert!(!ahead.is_compatible_with_host());
}

#[test]
fn abi_version_display() {
    let v = AbiVersion { major: 1, minor: 0 };
    assert_eq!(format!("{v}"), "1.0");
}

#[test]
fn compatibility_check_passes() {
    let caps = sample_capabilities();
    let result = check_compatibility(&caps, Some("1.0.0"), &[PluginCapability::Validate]);
    assert!(result.is_compatible());
}

#[test]
fn compatibility_check_schema_in_range() {
    let caps = sample_capabilities();
    let result = check_compatibility(&caps, Some("1.1.0"), &[]);
    assert!(result.is_compatible());
}

#[test]
fn compatibility_check_schema_out_of_range() {
    let caps = sample_capabilities();
    let result = check_compatibility(&caps, Some("2.0.0"), &[]);
    assert!(matches!(
        result,
        CompatibilityStatus::SchemaUnsupported { .. }
    ));
}

#[test]
fn semver_multi_digit_minor_accepted() {
    // Lexicographic comparison would reject "1.10.0" within ["1.0.0", "1.10.0"]
    // because "1.10.0" < "1.2.0" as strings. Semantic comparison must handle this.
    let caps = PluginCapabilities {
        supported_schemas: vec![SchemaVersionRange {
            min_version: "1.0.0".into(),
            max_version: "1.10.0".into(),
        }],
        capabilities: vec![],
        ..sample_capabilities()
    };
    let result = check_compatibility(&caps, Some("1.10.0"), &[]);
    assert!(
        result.is_compatible(),
        "1.10.0 must be accepted within [1.0.0, 1.10.0]"
    );
}

#[test]
fn semver_multi_digit_minor_rejected_correctly() {
    // "1.10.0" must be rejected when max is "1.2.0"
    let caps = PluginCapabilities {
        supported_schemas: vec![SchemaVersionRange {
            min_version: "1.0.0".into(),
            max_version: "1.2.0".into(),
        }],
        capabilities: vec![],
        ..sample_capabilities()
    };
    let result = check_compatibility(&caps, Some("1.10.0"), &[]);
    assert!(
        matches!(result, CompatibilityStatus::SchemaUnsupported { .. }),
        "1.10.0 must be rejected when max is 1.2.0"
    );
}

#[test]
fn compatibility_check_missing_capability() {
    let caps = sample_capabilities();
    let result = check_compatibility(&caps, None, &[PluginCapability::SubstanceScreening]);
    assert!(matches!(result, CompatibilityStatus::MissingCapability(_)));
}

#[test]
fn compatibility_check_abi_mismatch() {
    let mut caps = sample_capabilities();
    caps.abi_version = AbiVersion { major: 2, minor: 0 };
    let result = check_compatibility(&caps, None, &[]);
    assert!(matches!(
        result,
        CompatibilityStatus::AbiIncompatible { .. }
    ));
}

#[test]
fn compatibility_check_host_too_old() {
    let mut caps = sample_capabilities();
    caps.min_host_version = Some(AbiVersion {
        major: ABI_VERSION_MAJOR,
        minor: ABI_VERSION_MINOR + 5,
    });
    let result = check_compatibility(&caps, None, &[]);
    assert!(matches!(result, CompatibilityStatus::HostTooOld { .. }));
}

#[test]
fn compatibility_check_no_schema_constraint() {
    let caps = sample_capabilities();
    let result = check_compatibility(&caps, None, &[]);
    assert!(result.is_compatible());
}

#[test]
fn plugin_meta_round_trip() {
    let meta = PluginMeta {
        sector: "textile".into(),
        name: "Textile Compliance Plugin".into(),
        version: "0.2.0".into(),
        license: "Apache-2.0".into(),
        description: Some("Validates textile DPP data".into()),
        author: Some("Odal Node".into()),
        homepage: Some("https://github.com/odal-node".into()),
    };
    let json = serde_json::to_value(&meta).unwrap();
    assert_eq!(json["sector"], "textile");
    assert_eq!(json["description"], "Validates textile DPP data");
    let back: PluginMeta = serde_json::from_value(json).unwrap();
    assert_eq!(meta.name, back.name);
}

#[test]
fn capabilities_round_trip() {
    let caps = sample_capabilities();
    let json = serde_json::to_value(&caps).unwrap();
    assert!(json["supportedSchemas"].is_array());
    assert_eq!(json["abiVersion"]["major"], ABI_VERSION_MAJOR);
    let back: PluginCapabilities = serde_json::from_value(json).unwrap();
    assert_eq!(caps.abi_version, back.abi_version);
}

#[test]
fn plugin_field_error_round_trip() {
    let err = PluginFieldError {
        field: "/fibreComposition/0/pct".into(),
        code: "out_of_range".into(),
        message: "pct must be 0-100".into(),
    };
    let json = serde_json::to_value(&err).unwrap();
    assert_eq!(json["code"], "out_of_range");
    let back: PluginFieldError = serde_json::from_value(json).unwrap();
    assert_eq!(err.field, back.field);
}

#[test]
fn custom_capability_round_trip() {
    let cap = PluginCapability::Custom("carbon_offset_calc".into());
    let json = serde_json::to_value(&cap).unwrap();
    let back: PluginCapability = serde_json::from_value(json).unwrap();
    assert_eq!(cap, back);
}

#[test]
fn abi_result_ok_round_trip() {
    let result = PluginResult::new(PluginComplianceStatus::NotAssessed)
        .with_metric(METRIC_CO2E_SCORE, 85.4)
        .with_metric(METRIC_RECYCLED_CONTENT_PCT, 12.5);
    let envelope = AbiResult::ok(&result);
    assert!(envelope.is_ok());
    let json = serde_json::to_value(&envelope).unwrap();
    assert!(json["ok"].is_object());
    assert_eq!(json["ok"]["complianceStatus"], "NOT_ASSESSED");

    let back: AbiResult = serde_json::from_value(json).unwrap();
    match back {
        AbiResult::Ok(v) => assert_eq!(v["metrics"]["co2e_score"], 85.4),
        AbiResult::Error(_) => panic!("expected ok variant"),
    }
}

#[test]
fn abi_result_ok_rejects_non_finite_metric() {
    // A plugin can insert a non-finite metric directly into the pub `metrics`
    // field, bypassing the `with_metric` finite guard. Serialisation must fail,
    // and `AbiResult::ok` must surface that as an Error — not a spurious Ok
    // carrying a silently-nulled metric.
    let mut result = PluginResult::new(PluginComplianceStatus::Compliant);
    result
        .metrics
        .insert(METRIC_CO2E_SCORE.to_owned(), f64::INFINITY);
    let envelope = AbiResult::ok(&result);
    assert!(
        !envelope.is_ok(),
        "a non-finite metric must surface as an error, not Ok"
    );
}

#[test]
fn non_semver_range_bound_does_not_lexicographically_match() {
    // A range with an unparseable bound must not fall back to string comparison
    // (where "1.9.0" > "1.10.0"); the range simply can't match, so the check
    // fails closed rather than returning a misleading answer.
    let caps = PluginCapabilities {
        supported_schemas: vec![SchemaVersionRange {
            min_version: "1.0".into(), // not valid semver
            max_version: "1.10.0".into(),
        }],
        capabilities: vec![],
        ..sample_capabilities()
    };
    let result = check_compatibility(&caps, Some("1.9.0"), &[]);
    assert!(matches!(
        result,
        CompatibilityStatus::SchemaUnsupported { .. }
    ));
}

#[test]
fn abi_result_error_round_trip() {
    let envelope = AbiResult::Error(PluginError::ValidationErrors(vec![PluginFieldError {
        field: "/gtin".into(),
        code: "missing".into(),
        message: "gtin is required".into(),
    }]));
    assert!(!envelope.is_ok());
    let json = serde_json::to_value(&envelope).unwrap();
    assert!(json.get("error").is_some());

    let back: AbiResult = serde_json::from_value(json).unwrap();
    assert!(!back.is_ok());
}
