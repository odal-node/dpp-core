//! Glue round-trip tests plus `export_plugin!` macro-expansion coverage.

use super::*;
use crate::codec::to_bytes;
use dpp_plugin_traits::{
    AbiVersion, METRIC_CO2E_SCORE, PluginCapabilities, PluginComplianceStatus, PluginFieldError,
    PluginIdentity, PluginResult, SchemaVersionRange,
};
use serde_json::{Value, json};

/// Minimal plugin exercising every glue path.
#[derive(Default)]
struct DummyPlugin;

impl DppProductGroupPlugin for DummyPlugin {
    fn plugin_identity(&self) -> PluginIdentity {
        PluginIdentity {
            product_group: "dummy",
            name: "Dummy",
            version: "0.1.0",
            description: "Minimal plugin exercising every glue path",
        }
    }

    fn schema_version_range(&self) -> SchemaVersionRange {
        SchemaVersionRange {
            min_version: "1.0.0".into(),
            max_version: "1.0.0".into(),
        }
    }

    fn validate_input(&self, input: &PluginInput) -> Result<(), PluginError> {
        if input.get("ok").is_some() {
            Ok(())
        } else {
            Err(PluginError::ValidationErrors(vec![PluginFieldError {
                field: "/ok".into(),
                code: "missing".into(),
                message: "ok is required".into(),
            }]))
        }
    }

    fn calculate_metrics(&self, input: &PluginInput) -> Result<PluginResult, PluginError> {
        self.validate_input(input)?;
        Ok(PluginResult::new(PluginComplianceStatus::NotAssessed)
            .maybe_metric(METRIC_CO2E_SCORE, input.get("co2e").and_then(Value::as_f64)))
    }

    fn generate_passport(&self, input: PluginInput) -> Result<Value, PluginError> {
        self.validate_input(&input)?;
        Ok(input)
    }
}

fn parse(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).expect("glue emits valid JSON")
}

#[test]
fn describe_emits_capabilities() {
    let json = parse(&describe_bytes(&DummyPlugin));
    assert_eq!(json["abiVersion"]["major"], 1);
    assert!(json["supportedSchemas"].is_array());
    // Round-trips back into the typed contract the host uses.
    let back: PluginCapabilities = serde_json::from_value(json).unwrap();
    assert_eq!(back.abi_version, AbiVersion::current());
}

#[test]
fn metadata_emits_meta() {
    let json = parse(&metadata_bytes(&DummyPlugin));
    assert_eq!(json["productGroup"], "dummy");
}

#[test]
fn calculate_metrics_ok_envelope() {
    let input = json!({ "ok": true, "co2e": 42.0 });
    let json = parse(&calculate_metrics_bytes(&DummyPlugin, &to_bytes(&input)));
    assert_eq!(json["ok"]["metrics"]["co2e_score"], 42.0);
    assert_eq!(json["ok"]["complianceStatus"], "NOT_ASSESSED");
}

#[test]
fn calculate_metrics_validation_error_envelope() {
    let input = json!({ "co2e": 42.0 }); // missing "ok"
    let json = parse(&calculate_metrics_bytes(&DummyPlugin, &to_bytes(&input)));
    assert!(json.get("error").is_some());
    assert!(json.get("ok").is_none());
}

#[test]
fn validate_error_on_malformed_json() {
    let json = parse(&validate_bytes(&DummyPlugin, b"not json {{{"));
    let back: AbiResult = serde_json::from_value(json).unwrap();
    assert!(!back.is_ok());
}

#[test]
fn validate_ok_envelope_is_null() {
    let input = json!({ "ok": true });
    let json = parse(&validate_bytes(&DummyPlugin, &to_bytes(&input)));
    assert!(json["ok"].is_null());
}

#[test]
fn generate_passport_passthrough() {
    let input = json!({ "ok": true, "gtin": "12345678901231" });
    let json = parse(&generate_passport_bytes(&DummyPlugin, &to_bytes(&input)));
    assert_eq!(json["ok"]["gtin"], "12345678901231");
}

#[test]
fn validate_error_when_input_parses_but_is_rejected() {
    // Valid JSON, but DummyPlugin rejects it (missing "ok") — exercises the
    // parse-ok-but-validation-error arm, distinct from malformed JSON.
    let input = json!({ "missing": "ok" });
    let json = parse(&validate_bytes(&DummyPlugin, &to_bytes(&input)));
    assert!(json.get("error").is_some());
    assert!(json.get("ok").is_none());
}

#[test]
fn generate_passport_error_when_input_parses_but_is_rejected() {
    let input = json!({ "missing": "ok" });
    let json = parse(&generate_passport_bytes(&DummyPlugin, &to_bytes(&input)));
    assert!(json.get("error").is_some());
    assert!(json.get("ok").is_none());
}

// Note: the `write_output`/`read_input`/`host_alloc` pointer packing is a
// 32-bit ABI. On a 64-bit host the raw functions guard against truncation —
// `host_alloc`/`write_output` return null instead of a truncated pointer and
// `read_input` never dereferences one — so the macro exports below are memory-
// safe to drive on the host. The primary host-testable surface remains the pure
// `*_bytes` glue exercised above.

// ── `export_plugin!` macro expansion (host-target coverage) ──────────────
//
// The macro generates `extern "C"` wrappers that the host calls across the
// Wasm boundary. We can exercise the *expansion itself* on the host without
// a Wasm runtime: every wrapper delegates to host-testable glue, and the
// input-taking exports are driven with `len == 0`, which `read_input`
// short-circuits to an empty slice — so no host pointer is ever
// dereferenced. The 32-bit pointer truncation only affects the packed
// `out_ptr` high bits; the `out_len` low 32 bits are exact, so we assert the
// wrapper packs the same buffer length the glue produces.
export_plugin!(DummyPlugin);

/// Low 32 bits of a packed `(out_ptr << 32) | out_len` ABI return.
fn out_len(packed: u64) -> usize {
    (packed & 0xFFFF_FFFF) as usize
}

#[test]
fn macro_alloc_dealloc_are_callable() {
    // Zero-length alloc returns a null pointer without allocating.
    assert_eq!(alloc(0), 0);
    // Non-zero alloc: on a 64-bit host the address can't fit the 32-bit ABI, so
    // host_alloc frees it and returns null rather than a truncated pointer.
    let _ = alloc(8);
    // dealloc's null/zero guard is the only branch safe to drive on host.
    dealloc(0, 0);
}

#[test]
#[cfg(not(target_pointer_width = "32"))]
fn read_input_never_dereferences_truncated_pointer_on_host() {
    // On a 64-bit host a 32-bit ABI pointer is a truncated address; read_input
    // must return an empty slice rather than dereferencing it, so a plugin's
    // native test suite cannot trigger memory unsafety through the raw ABI.
    let bytes = unsafe { crate::abi::read_input(0xDEAD_BEEF, 16) };
    assert!(
        bytes.is_empty(),
        "must not deref a truncated pointer on host"
    );
}

#[test]
#[cfg(not(target_pointer_width = "32"))]
fn host_alloc_does_not_hand_out_truncated_pointer_on_host() {
    // A real 64-bit heap address cannot be represented in u32, so host_alloc
    // returns null instead of a truncated (un-freeable) pointer.
    assert_eq!(crate::abi::host_alloc(8), 0);
}

#[test]
fn macro_metadata_and_describe_pack_glue_output() {
    assert_eq!(out_len(metadata()), metadata_bytes(&DummyPlugin).len());
    assert_eq!(out_len(describe()), describe_bytes(&DummyPlugin).len());
}

#[test]
fn macro_input_exports_pack_error_envelope_for_empty_input() {
    // `(ptr, 0)` → read_input yields `&[]` (it short-circuits on len == 0
    // and never dereferences the pointer) → parse error → Error envelope.
    assert_eq!(
        out_len(validate(0, 0)),
        validate_bytes(&DummyPlugin, &[]).len()
    );
    assert_eq!(
        out_len(calculate_metrics(0, 0)),
        calculate_metrics_bytes(&DummyPlugin, &[]).len()
    );
    assert_eq!(
        out_len(generate_passport(0, 0)),
        generate_passport_bytes(&DummyPlugin, &[]).len()
    );
}

// ── require_product_identifier ───────────────────────────────────────────────
//
// The check that replaced `require_gtin` for product group data. Each scheme is
// exercised on its own, because the whole point of clause 5 is that the three
// are alternatives: a check that only ever passes for GS1 is the defect this
// method exists to remove.

fn pi_errors(identifier: Value) -> Vec<(String, String)> {
    let input = json!({ "productIdentifier": identifier });
    match crate::validate::Validator::new(&input)
        .require_product_identifier("productIdentifier")
        .finish()
    {
        Ok(()) => vec![],
        Err(PluginError::ValidationErrors(errors)) => {
            errors.into_iter().map(|e| (e.field, e.code)).collect()
        }
        Err(other) => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn every_clause_5_scheme_is_accepted() {
    for identifier in [
        json!({ "scheme": "gs1", "gtin": "09506000134352" }),
        json!({ "scheme": "identificationLink", "url": "https://id.acme.example.com/p/1" }),
        json!({ "scheme": "did", "did": "did:web:acme.example.com:p:1" }),
    ] {
        assert!(
            pi_errors(identifier.clone()).is_empty(),
            "scheme 2 and 3 carry no GTIN and must still pass: {identifier}"
        );
    }
}

#[test]
fn a_scheme_2_identifier_is_not_asked_for_a_gtin() {
    // The regression that mattered: requiring a GTIN unconditionally rejected
    // exactly the passports the identifier work introduced.
    let errors = pi_errors(json!({ "scheme": "did", "did": "did:web:acme.example.com" }));
    assert!(
        !errors.iter().any(|(field, _)| field.contains("gtin")),
        "a DID-identified product was asked for a GTIN: {errors:?}"
    );
}

#[test]
fn the_branch_field_is_required_by_its_own_scheme() {
    assert_eq!(
        pi_errors(json!({ "scheme": "gs1" })),
        vec![("/productIdentifier/gtin".to_owned(), "missing".to_owned())]
    );
    assert_eq!(
        pi_errors(json!({ "scheme": "identificationLink" })),
        vec![("/productIdentifier/url".to_owned(), "missing".to_owned())]
    );
    assert_eq!(
        pi_errors(json!({ "scheme": "did" })),
        vec![("/productIdentifier/did".to_owned(), "missing".to_owned())]
    );
}

#[test]
fn a_malformed_branch_value_is_refused_per_scheme() {
    // GS1 keeps the check-digit test the bare `gtin` field used to get.
    assert_eq!(
        pi_errors(json!({ "scheme": "gs1", "gtin": "09506000134353" })),
        vec![("/productIdentifier/gtin".to_owned(), "checksum".to_owned())]
    );
    assert_eq!(
        pi_errors(json!({ "scheme": "gs1", "gtin": "12-34" })),
        vec![("/productIdentifier/gtin".to_owned(), "format".to_owned())]
    );
    assert_eq!(
        pi_errors(json!({ "scheme": "identificationLink", "url": "acme.example.com" })),
        vec![("/productIdentifier/url".to_owned(), "format".to_owned())]
    );
    assert_eq!(
        pi_errors(json!({ "scheme": "did", "did": "did:key:z6Mk" })),
        vec![("/productIdentifier/did".to_owned(), "format".to_owned())]
    );
}

#[test]
fn a_carrier_shaped_value_with_nothing_to_resolve_is_refused() {
    // 🚨 These four passed. The plugin tier tested a prefix and a non-empty
    // remainder, so a URL with no authority and a DID with a space in it were
    // both accepted here while `dpp_domain::ProductIdentifier` refused them —
    // and the plugin is the *first* thing to see product group data, so the
    // weaker of the two copies was the one on the outside. Both tiers now call
    // `dpp_rules::common::identifier`.
    for url in [
        "https:///acme/1",
        "https://?q=1",
        "https://",
        "https://ac me.example.com",
    ] {
        assert_eq!(
            pi_errors(json!({ "scheme": "identificationLink", "url": url })),
            vec![("/productIdentifier/url".to_owned(), "format".to_owned())],
            "{url} has no host to resolve"
        );
    }
    for did in [
        "did:web: ",
        "did:web:",
        "did:web:acme:",
        "did:web:ac%2zme",
        "did:web",
    ] {
        assert_eq!(
            pi_errors(json!({ "scheme": "did", "did": did })),
            vec![("/productIdentifier/did".to_owned(), "format".to_owned())],
            "{did} is not a resolvable DID"
        );
    }
}

#[test]
fn an_unmapped_scheme_is_refused_rather_than_skipped() {
    // 🚨 The `passport_id` case. A scheme nobody has mapped is where an
    // invented identifier passes unexamined, so it must fail rather than
    // fall through to "no branch to check".
    assert_eq!(
        pi_errors(json!({ "scheme": "passport_id", "value": "0199...uuid" })),
        vec![("/productIdentifier/scheme".to_owned(), "unknown".to_owned())]
    );
}

#[test]
fn an_absent_or_headless_identifier_is_refused() {
    let empty = json!({});
    let missing = match crate::validate::Validator::new(&empty)
        .require_product_identifier("productIdentifier")
        .finish()
    {
        Err(PluginError::ValidationErrors(e)) => e,
        other => panic!("expected a refusal, got {other:?}"),
    };
    assert_eq!(missing[0].field, "/productIdentifier");
    assert_eq!(missing[0].code, "missing");

    assert_eq!(
        pi_errors(json!({ "gtin": "09506000134352" })),
        vec![("/productIdentifier/scheme".to_owned(), "missing".to_owned())],
        "a GTIN with no scheme states nothing about which scheme issued it"
    );
}
