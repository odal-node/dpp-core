# dpp-plugin-sdk

[![crates.io](https://img.shields.io/crates/v/dpp-plugin-sdk.svg)](https://crates.io/crates/dpp-plugin-sdk)
[![docs.rs](https://img.shields.io/docsrs/dpp-plugin-sdk)](https://docs.rs/dpp-plugin-sdk)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](../../LICENSE)

Guest-side SDK and ABI export macro for the [Odal Node](https://odal-node.io) Wasm sector plugins.
This crate provides the glue and helper utilities that make writing a Wasm sector plugin
straightforward: a single `export_plugin!` macro generates the linear-memory
ABI (`alloc`/`dealloc`) and the standard exports (`metadata`, `describe`,
`validate`, `calculate_metrics`, `generate_passport`).

## When to use this crate

- You are authoring a Wasm sector plugin and want the standard host/guest ABI
  wiring without hand-rolling alloc/dealloc and JSON packing.
- You need the helper functions that serialise/deserialise the `Plugin*` types
  defined in `dpp-plugin-traits` and want to reuse the same `dpp_rules`
  implementation as the host.

## Example

```rust
use dpp_plugin_sdk::traits::*;

#[derive(Default)]
struct BatteryPlugin;

impl DppSectorPlugin for BatteryPlugin {
    fn plugin_identity(&self) -> PluginIdentity {
        todo!("sector key, plugin name and semantic version")
    }
    fn schema_version_range(&self) -> SchemaVersionRange {
        todo!("the schema versions this plugin accepts")
    }
    fn validate_input(&self, input: &PluginInput) -> Result<(), PluginError> {
        todo!("cross-field rules from dpp-rules")
    }
    fn calculate_metrics(&self, input: &PluginInput) -> Result<PluginResult, PluginError> {
        todo!("compliance determination")
    }
    fn generate_passport(&self, input: PluginInput) -> Result<serde_json::Value, PluginError> {
        todo!("the sector payload")
    }
}

// `meta` and `capabilities` have default implementations — override them only
// if the defaults are wrong for your sector.
//
// Then call the macro exactly once at the crate root to generate the ABI:
//
//     dpp_plugin_sdk::export_plugin!(BatteryPlugin);
```

`plugins/sector-battery` is the reference implementation — read it before writing a new one.

## Relationship to other crates

| Crate | Role |
|---|---|
| `dpp-plugin-traits` | Defines the host/guest contract (`PluginMeta`, `PluginCapabilities`, `DppSectorPlugin`) |
| `dpp-rules` | Re-exported rule implementations so plugins share the same rules engine as the host |

## Minimum Rust version

1.96 (MSRV is enforced in CI)

## License

Apache-2.0 — see [LICENSE](../../LICENSE)
