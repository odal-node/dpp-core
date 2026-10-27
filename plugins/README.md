# Product group plugins

Wasm plugins that implement product-group-specific compliance logic for EU Digital
Product Passports. There is one plugin per **product group**, and all eleven are built
with `dpp-plugin-sdk`. `product-group-battery` is the reference implementation.

## Product group and product category

A **product group** is the EU delegated-act bucket a host dispatches on (`battery`,
`textile`, `steel` and so on). It selects both the schema version and the plugin. A
**product category** (`ev` or `portable` for batteries, `smartphone` for electronics) is
data inside the product group that a plugin may branch on; it is **never** a dispatch
key. One product group, one plugin, many product categories. See
`docs/architecture/DATA-MODEL.md` §3.4.

## Build

```bash
rustup target add wasm32-wasip1
just build-plugins
```

The plugins share one workspace (`plugins/Cargo.toml`), separate from the library
workspace because they compile to `wasm32-wasip1`. Cargo writes the artifacts to
`plugins/target/wasm32-wasip1/release/product_group_<name>.wasm`. How a host locates and
loads them is up to the host; check its documentation for the file name it expects.

## Write a plugin

A plugin implements `DppProductGroupPlugin` from `dpp-plugin-traits` and calls
`export_plugin!` once. The macro generates the whole Wasm ABI, so you never write
`alloc`, `dealloc` or output structs by hand.

```rust
use dpp_plugin_sdk::{export_plugin, traits::*};
use serde_json::Value;

#[derive(Default)]
struct MyPlugin;

impl DppProductGroupPlugin for MyPlugin {
    fn plugin_identity(&self) -> PluginIdentity { /* product group key, name, version, description */ }
    fn schema_version_range(&self) -> SchemaVersionRange { /* supported schema versions */ }
    fn validate_input(&self, input: &PluginInput) -> Result<(), PluginError> { /* field checks */ }
    fn calculate_metrics(&self, input: &PluginInput) -> Result<PluginResult, PluginError> { /* metrics */ }
    fn generate_passport(&self, input: PluginInput) -> Result<Value, PluginError> { /* normalise */ }
    // meta()/capabilities() default to PluginIdentity + schema_version_range() above.
    // Override them only if this plugin needs other values.
}

export_plugin!(MyPlugin);
```

**Inside this repository:** add `plugins/product-group-<name>/`, copy
`product-group-battery/Cargo.toml` (it inherits version, edition, licence and
dependencies from the plugin workspace), and add the directory to `members` in
`plugins/Cargo.toml`. If the product group is new, add its manifest and its first schema
under `crates/dpp-domain/` as described in [CONTRIBUTING.md](../CONTRIBUTING.md).

**In your own repository:** create a `cdylib` crate and depend on `dpp-plugin-sdk` and
`serde_json` from crates.io:

```toml
[package]
name = "product-group-myname"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[dependencies]
dpp-plugin-sdk = "0.22"
serde_json     = "1"
```

Plugins must stay free of I/O and async runtimes: no `tokio`, `reqwest`, `sqlx` or
similar anywhere in the dependency tree, including for tests.

## Generated ABI

| Symbol | Signature | Returns (JSON) |
|--------|-----------|----------------|
| `alloc` | `(len: u32) -> u32` | pointer to `len` bytes |
| `dealloc` | `(ptr: u32, len: u32)` | nothing |
| `metadata` | `() -> u64` | `PluginMeta` |
| `describe` | `() -> u64` | `PluginCapabilities` (the host runs `check_compatibility` before dispatch) |
| `validate` | `(ptr, len) -> u64` | `AbiResult` (`{ "ok": null }` or `{ "error": … }`) |
| `calculate_metrics` | `(ptr, len) -> u64` | `AbiResult` (`{ "ok": PluginResult }`) |
| `generate_passport` | `(ptr, len) -> u64` | `AbiResult` (`{ "ok": payload }`) |

Each `-> u64` packs the output as `(out_ptr << 32) | out_len`. Input and output are UTF-8
JSON in linear memory.

## Plugins in this directory

`product-group-aluminium`, `-battery`, `-construction`, `-detergent`, `-electronics`,
`-furniture`, `-mattress`, `-steel`, `-textile`, `-toy` and `-tyre`. Every one uses the
SDK; none uses the older hand-written ABI.
