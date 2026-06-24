//! Guest-side SDK for Odal Node Wasm sector plugins.
//!
//! A sector plugin author implements the [`DppSectorPlugin`] trait from
//! `dpp-plugin-traits` and invokes [`export_plugin!`] once. The macro generates
//! the full low-level Wasm ABI (`alloc`, `dealloc`, `metadata`, `describe`,
//! `validate`, `calculate_metrics`, `generate_passport`) and wires each export
//! to the corresponding trait method. Plugins no longer hand-roll the ABI shim
//! or redefine their own output structs — they speak the shared contract.
//!
//! ## Why `describe()`
//!
//! The host calls `describe()` immediately after loading a plugin to read its
//! [`PluginCapabilities`](dpp_plugin_traits::PluginCapabilities) (ABI version, supported schema versions, feature
//! capabilities) and run `dpp_plugin_traits::check_compatibility` *before*
//! dispatching any work. This is what makes the version registry enforceable at
//! the Wasm boundary rather than aspirational.
//!
//! ## ABI summary
//!
//! | Export | Signature | Returns (JSON) |
//! |--------|-----------|----------------|
//! | `alloc` | `(len: u32) -> u32` | pointer to `len` bytes |
//! | `dealloc` | `(ptr: u32, len: u32)` | — |
//! | `metadata` | `() -> u64` | [`PluginMeta`](dpp_plugin_traits::PluginMeta) |
//! | `describe` | `() -> u64` | [`PluginCapabilities`](dpp_plugin_traits::PluginCapabilities) |
//! | `validate` | `(ptr: u32, len: u32) -> u64` | [`AbiResult`] (`ok: null` / `error`) |
//! | `calculate_metrics` | `(ptr: u32, len: u32) -> u64` | [`AbiResult`] (`ok: PluginResult`) |
//! | `generate_passport` | `(ptr: u32, len: u32) -> u64` | [`AbiResult`] (`ok: payload`) |
//!
//! Every `-> u64` return packs the output buffer as `(out_ptr << 32) | out_len`.
//! The host reads the JSON, then frees the buffer via `dealloc`.

/// Re-export of the shared host/guest contract so plugins need only one
/// path dependency.
pub use dpp_plugin_traits as traits;

/// Re-export of the shared cross-field regulatory rules ([`dpp_rules`]), so a
/// plugin uses the same rule implementation as `dpp-domain` rather than
/// reimplementing it.
pub use dpp_rules as rules;

pub mod validate;

use dpp_plugin_traits::{AbiResult, DppSectorPlugin, PluginError, PluginInput};
pub use dpp_plugin_traits::{
    METRIC_CO2E_SCORE, METRIC_RECYCLED_CONTENT_PCT, METRIC_REPAIRABILITY_INDEX,
    PluginComplianceStatus,
};
use serde::Serialize;

// ─── Low-level linear-memory ABI ──────────────────────────────────────────────

pub mod abi {
    use std::alloc::{Layout, alloc as mem_alloc, dealloc as mem_dealloc};

    /// Allocate `len` bytes in the module's linear memory and return the
    /// pointer as a `u32`. Returns `0` for a zero-length request.
    #[must_use]
    pub fn host_alloc(len: u32) -> u32 {
        if len == 0 {
            return 0;
        }
        let layout = Layout::from_size_align(len as usize, 1).expect("valid layout");
        // SAFETY: `layout` has non-zero size; a null return is handled by the host.
        unsafe { mem_alloc(layout) as u32 }
    }

    /// Free a buffer previously returned by [`host_alloc`] (or packed into a
    /// `-> u64` ABI return). No-op for null pointers or zero length.
    pub fn host_dealloc(ptr: u32, len: u32) {
        if ptr == 0 || len == 0 {
            return;
        }
        let layout = Layout::from_size_align(len as usize, 1).expect("valid layout");
        // SAFETY: `ptr`/`len` must describe a buffer from `host_alloc`.
        unsafe { mem_dealloc(ptr as *mut u8, layout) }
    }

    /// View the host-written input buffer as a byte slice.
    ///
    /// # Safety
    ///
    /// `ptr` and `len` must describe a single allocation written by the host
    /// (via `alloc`) that lives for the duration of the returned borrow.
    #[must_use]
    pub unsafe fn read_input<'a>(ptr: u32, len: u32) -> &'a [u8] {
        unsafe {
            if len == 0 {
                return &[];
            }
            std::slice::from_raw_parts(ptr as *const u8, len as usize)
        }
    }

    /// Leak `bytes` into linear memory and return the packed
    /// `(ptr << 32) | len` the host uses to read and later free it.
    ///
    /// The buffer is shrunk to an exact-size allocation (`capacity == len`,
    /// align 1) so that the host's `dealloc(ptr, len)` frees precisely the
    /// allocation it was given. Returning a `Vec` directly would leak its
    /// (possibly larger) capacity and make `dealloc` a size-mismatched free.
    #[must_use]
    pub fn write_output(bytes: Vec<u8>) -> u64 {
        let mut boxed = bytes.into_boxed_slice();
        let out_len = boxed.len() as u32;
        let out_ptr = boxed.as_mut_ptr() as usize as u32;
        std::mem::forget(boxed);
        ((out_ptr as u64) << 32) | (out_len as u64)
    }
}

// ─── Pure glue (host-testable, no linear-memory side effects) ─────────────────

fn to_bytes<T: Serialize>(value: &T) -> Vec<u8> {
    serde_json::to_vec(value).unwrap_or_default()
}

fn parse_input(bytes: &[u8]) -> Result<PluginInput, PluginError> {
    serde_json::from_slice(bytes).map_err(|e| PluginError::InvalidInput(e.to_string()))
}

/// Serialise the plugin's [`PluginMeta`](dpp_plugin_traits::PluginMeta) to JSON bytes.
pub fn metadata_bytes<P: DppSectorPlugin>(plugin: &P) -> Vec<u8> {
    to_bytes(&plugin.meta())
}

/// Serialise the plugin's [`PluginCapabilities`](dpp_plugin_traits::PluginCapabilities) to JSON bytes.
pub fn describe_bytes<P: DppSectorPlugin>(plugin: &P) -> Vec<u8> {
    to_bytes(&plugin.capabilities())
}

/// Run `validate_input` and serialise the [`AbiResult`] envelope.
pub fn validate_bytes<P: DppSectorPlugin>(plugin: &P, input: &[u8]) -> Vec<u8> {
    let outcome = match parse_input(input) {
        Ok(value) => match plugin.validate_input(&value) {
            Ok(()) => AbiResult::Ok(serde_json::Value::Null),
            Err(e) => AbiResult::Error(e),
        },
        Err(e) => AbiResult::Error(e),
    };
    to_bytes(&outcome)
}

/// Run `calculate_metrics` and serialise the [`AbiResult`] envelope.
pub fn calculate_metrics_bytes<P: DppSectorPlugin>(plugin: &P, input: &[u8]) -> Vec<u8> {
    let outcome = match parse_input(input) {
        Ok(value) => match plugin.calculate_metrics(&value) {
            Ok(result) => AbiResult::ok(&result),
            Err(e) => AbiResult::Error(e),
        },
        Err(e) => AbiResult::Error(e),
    };
    to_bytes(&outcome)
}

/// Run `generate_passport` and serialise the [`AbiResult`] envelope.
pub fn generate_passport_bytes<P: DppSectorPlugin>(plugin: &P, input: &[u8]) -> Vec<u8> {
    let outcome = match parse_input(input) {
        Ok(value) => match plugin.generate_passport(&value) {
            Ok(payload) => AbiResult::Ok(payload),
            Err(e) => AbiResult::Error(e),
        },
        Err(e) => AbiResult::Error(e),
    };
    to_bytes(&outcome)
}

// ─── ABI entry-point wrappers (called by `export_plugin!`) ────────────────────

pub fn run_metadata<P: DppSectorPlugin>(plugin: &P) -> u64 {
    abi::write_output(metadata_bytes(plugin))
}

pub fn run_describe<P: DppSectorPlugin>(plugin: &P) -> u64 {
    abi::write_output(describe_bytes(plugin))
}

/// # Safety
/// `ptr`/`len` must describe a host-written input buffer (see [`abi::read_input`]).
pub unsafe fn run_validate<P: DppSectorPlugin>(plugin: &P, ptr: u32, len: u32) -> u64 {
    unsafe { abi::write_output(validate_bytes(plugin, abi::read_input(ptr, len))) }
}

/// # Safety
/// `ptr`/`len` must describe a host-written input buffer (see [`abi::read_input`]).
pub unsafe fn run_calculate_metrics<P: DppSectorPlugin>(plugin: &P, ptr: u32, len: u32) -> u64 {
    unsafe { abi::write_output(calculate_metrics_bytes(plugin, abi::read_input(ptr, len))) }
}

/// # Safety
/// `ptr`/`len` must describe a host-written input buffer (see [`abi::read_input`]).
pub unsafe fn run_generate_passport<P: DppSectorPlugin>(plugin: &P, ptr: u32, len: u32) -> u64 {
    unsafe { abi::write_output(generate_passport_bytes(plugin, abi::read_input(ptr, len))) }
}

// ─── Export macro ─────────────────────────────────────────────────────────────

/// Generate the full Wasm ABI for a sector plugin.
///
/// `$plugin` must implement [`DppSectorPlugin`](dpp_plugin_traits::DppSectorPlugin)
/// and [`Default`] (plugins are deterministic and stateless, so the instance is
/// constructed per call). Invoke once at the crate root:
///
/// ```ignore
/// use dpp_plugin_sdk::{export_plugin, traits::*};
///
/// #[derive(Default)]
/// struct BatteryPlugin;
/// impl DppSectorPlugin for BatteryPlugin { /* ... */ }
///
/// export_plugin!(BatteryPlugin);
/// ```
#[macro_export]
macro_rules! export_plugin {
    ($plugin:ty) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn alloc(len: u32) -> u32 {
            $crate::abi::host_alloc(len)
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn dealloc(ptr: u32, len: u32) {
            $crate::abi::host_dealloc(ptr, len)
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn metadata() -> u64 {
            $crate::run_metadata(&<$plugin as ::core::default::Default>::default())
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn describe() -> u64 {
            $crate::run_describe(&<$plugin as ::core::default::Default>::default())
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn validate(ptr: u32, len: u32) -> u64 {
            // SAFETY: the host guarantees `ptr`/`len` describe a buffer it wrote via `alloc`.
            unsafe {
                $crate::run_validate(&<$plugin as ::core::default::Default>::default(), ptr, len)
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn calculate_metrics(ptr: u32, len: u32) -> u64 {
            // SAFETY: the host guarantees `ptr`/`len` describe a buffer it wrote via `alloc`.
            unsafe {
                $crate::run_calculate_metrics(
                    &<$plugin as ::core::default::Default>::default(),
                    ptr,
                    len,
                )
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn generate_passport(ptr: u32, len: u32) -> u64 {
            // SAFETY: the host guarantees `ptr`/`len` describe a buffer it wrote via `alloc`.
            unsafe {
                $crate::run_generate_passport(
                    &<$plugin as ::core::default::Default>::default(),
                    ptr,
                    len,
                )
            }
        }
    };
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use dpp_plugin_traits::{
        AbiVersion, METRIC_CO2E_SCORE, PluginCapabilities, PluginCapability,
        PluginComplianceStatus, PluginFieldError, PluginMeta, PluginResult, SchemaVersionRange,
    };
    use serde_json::{Value, json};

    /// Minimal plugin exercising every glue path.
    #[derive(Default)]
    struct DummyPlugin;

    impl DppSectorPlugin for DummyPlugin {
        fn meta(&self) -> PluginMeta {
            PluginMeta {
                sector: "dummy".into(),
                name: "Dummy".into(),
                version: "0.1.0".into(),
                license: "Apache-2.0".into(),
                description: None,
                author: None,
                homepage: None,
            }
        }

        fn capabilities(&self) -> PluginCapabilities {
            PluginCapabilities {
                abi_version: AbiVersion::current(),
                supported_schemas: vec![SchemaVersionRange {
                    min_version: "1.0.0".into(),
                    max_version: "1.0.0".into(),
                }],
                capabilities: vec![PluginCapability::ComputeMetrics],
                min_host_version: None,
                max_fuel: None,
                max_memory_bytes: None,
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

        fn generate_passport(&self, input: &PluginInput) -> Result<Value, PluginError> {
            self.validate_input(input)?;
            Ok(input.clone())
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

    // Note: the `write_output`/`read_input` packing uses 32-bit pointers and is
    // only valid on `wasm32` (host pointers are 64-bit and would truncate). The
    // host-testable surface is the pure `*_bytes` glue exercised above.

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
        // Non-zero alloc returns a (truncated-on-host) pointer; the allocation
        // is intentionally leaked — the truncated u32 cannot be safely freed on
        // a 64-bit host, and dealloc's no-op path is covered below.
        let _ = alloc(8);
        // dealloc's null/zero guard is the only branch safe to drive on host.
        dealloc(0, 0);
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
}
