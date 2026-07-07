//! Guest-side SDK for Odal Node Wasm sector plugins.
//!
//! A sector plugin author implements the [`DppSectorPlugin`](dpp_plugin_traits::DppSectorPlugin) trait from
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
//! | `validate` | `(ptr: u32, len: u32) -> u64` | [`AbiResult`](dpp_plugin_traits::AbiResult) (`ok: null` / `error`) |
//! | `calculate_metrics` | `(ptr: u32, len: u32) -> u64` | [`AbiResult`](dpp_plugin_traits::AbiResult) (`ok: PluginResult`) |
//! | `generate_passport` | `(ptr: u32, len: u32) -> u64` | [`AbiResult`](dpp_plugin_traits::AbiResult) (`ok: payload`) |
//!
//! Every `-> u64` return packs the output buffer as `(out_ptr << 32) | out_len`.
//! The host reads the JSON, then frees the buffer via `dealloc`.
//!
//! ## Module layout
//!
//! - [`abi`] — the low-level linear-memory ABI (`alloc`/`dealloc`/buffer packing).
//! - `codec` — pure, host-testable JSON glue (`*_bytes` functions).
//! - `entry` — the `run_*` ABI entry-point wrappers `export_plugin!` calls.
//! - [`validate`] — shared field-validation helpers.

/// Re-export of the shared host/guest contract so plugins need only one
/// path dependency.
pub use dpp_plugin_traits as traits;

/// Re-export of the shared cross-field regulatory rules ([`dpp_rules`]), so a
/// plugin uses the same rule implementation as `dpp-domain` rather than
/// reimplementing it.
pub use dpp_rules as rules;

pub mod abi;
mod codec;
mod entry;
#[cfg(test)]
mod tests;
pub mod validate;

#[cfg(test)]
use dpp_plugin_traits::{AbiResult, DppSectorPlugin, PluginError, PluginInput};
pub use dpp_plugin_traits::{
    METRIC_CO2E_SCORE, METRIC_RECYCLED_CONTENT_PCT, METRIC_REPAIRABILITY_INDEX,
    PluginComplianceStatus,
};

pub use codec::{
    calculate_metrics_bytes, describe_bytes, generate_passport_bytes, metadata_bytes,
    validate_bytes,
};
pub use entry::{
    run_calculate_metrics, run_describe, run_generate_passport, run_metadata, run_validate,
};

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
