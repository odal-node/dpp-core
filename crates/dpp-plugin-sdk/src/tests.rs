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

impl DppSectorPlugin for DummyPlugin {
    fn plugin_identity(&self) -> PluginIdentity {
        PluginIdentity {
            sector: "dummy",
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
    assert_eq!(json["sector"], "dummy");
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
