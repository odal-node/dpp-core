#![no_main]
//! Fuzz the host's deserialization of plugin-emitted ABI envelopes — a
//! sandboxed-but-not-fully-trusted Wasm plugin's output is the byte frontier
//! here (see `docs/architecture/PLUGIN-HOST.md`).
//! Property: parsing `AbiResult`/`PluginCapabilities` JSON returns `Ok`/`Err`
//! for any input, never panics.

use dpp_plugin_traits::{AbiResult, PluginCapabilities};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = serde_json::from_slice::<AbiResult>(data);
    let _ = serde_json::from_slice::<PluginCapabilities>(data);
});
