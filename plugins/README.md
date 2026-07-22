# Odal Node Sector Plugins

Wasm sector plugins implementing the EU ESPR compliance logic per **sector**.

## Sector vs. product category

A **sector** is the EU delegated-act bucket the host dispatches on (`battery`, `textile`, `steel`, …) — it selects both the schema version and the plugin. A **product category** (e.g. `ev`/`portable` for batteries, `smartphone` for electronics) is *sector data* a plugin may branch on internally; it is **never** a dispatch key. One sector → one plugin → many product categories. See `docs/architecture/DATA-MODEL.md` §3.5.

## Building

```bash
# Requires the wasm32-wasip1 target:
rustup target add wasm32-wasip1

# Build one plugin:
cd plugins/sector-battery && cargo build --target wasm32-wasip1 --release
# Output: target/wasm32-wasip1/release/sector_battery.wasm

# Or build all at once from the repo root:
just build-plugins
```

Copy the `.wasm` into the node's `PLUGINS_DIR` (default: `./plugins/`). The sector key is the filename stem stripped of the `sector-` prefix.

## Writing a plugin (SDK)

Plugins implement the `DppSectorPlugin` trait from `dpp-plugin-traits` and call `export_plugin!` once. The `dpp-plugin-sdk` macro generates the entire Wasm ABI — authors never hand-roll `alloc`/`dealloc` or output structs.

```rust
use dpp_plugin_sdk::{export_plugin, traits::*};
use serde_json::Value;

#[derive(Default)]
struct MyPlugin;

impl DppSectorPlugin for MyPlugin {
    fn plugin_identity(&self) -> PluginIdentity { /* sector key, name, version, description */ }
    fn schema_version_range(&self) -> SchemaVersionRange { /* supported schema versions */ }
    fn validate_input(&self, input: &PluginInput) -> Result<(), PluginError> { /* field checks */ }
    fn calculate_metrics(&self, input: &PluginInput) -> Result<PluginResult, PluginError> { /* metrics */ }
    fn generate_passport(&self, input: PluginInput) -> Result<Value, PluginError> { /* normalise */ }
    // meta()/capabilities() default to PluginIdentity + schema_version_range() above —
    // override only if this plugin needs non-default values.
}

export_plugin!(MyPlugin);
```

`Cargo.toml`:

```toml
[workspace]            # detach from the dpp-core workspace

[package]
name = "sector-myname"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[dependencies]
dpp-plugin-sdk = { path = "../../crates/dpp-plugin-sdk" }
serde_json     = "1"
```

Add the JSON schema at `crates/dpp-domain/schemas/{sector}/v1.0.0.json` — the `VersionedSchemaRegistry` embeds it at compile time. **`sector-battery` is the reference implementation.**

## Generated ABI

| Symbol | Signature | Returns (JSON) |
|--------|-----------|----------------|
| `alloc` | `(len: u32) -> u32` | pointer to `len` bytes |
| `dealloc` | `(ptr: u32, len: u32)` | — |
| `metadata` | `() -> u64` | `PluginMeta` |
| `describe` | `() -> u64` | `PluginCapabilities` (host runs `check_compatibility` before dispatch) |
| `validate` | `(ptr, len) -> u64` | `AbiResult` (`{ "ok": null }` / `{ "error": … }`) |
| `calculate_metrics` | `(ptr, len) -> u64` | `AbiResult` (`{ "ok": PluginResult }`) |
| `generate_passport` | `(ptr, len) -> u64` | `AbiResult` (`{ "ok": payload }`) |

Each `-> u64` packs the output as `(out_ptr << 32) | out_len`. Input/output is UTF-8 JSON over linear memory.

## SDK status

All ten plugins run on the SDK: each depends on `dpp-plugin-sdk`, implements `DppSectorPlugin`, and calls `export_plugin!` once — `sector-battery`, `textile`, `steel`, `electronics`, `construction`, `tyre`, `toy`, `aluminium`, `furniture`, and `detergent`. None hand-roll the legacy 3-symbol ABI. `sector-battery` is the reference implementation.
