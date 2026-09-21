# dpp-plugin-sdk

[![crates.io](https://img.shields.io/crates/v/dpp-plugin-sdk.svg)](https://crates.io/crates/dpp-plugin-sdk)
[![docs.rs](https://img.shields.io/docsrs/dpp-plugin-sdk)](https://docs.rs/dpp-plugin-sdk)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](../../LICENSE)

Guest-side SDK and ABI export macro for the [Odal Node](https://odal-node.io) Wasm product group plugins.
This crate provides the glue and helper utilities that make writing a Wasm product group plugin
straightforward: a single `export_plugin!` macro generates the linear-memory
ABI (`alloc`/`dealloc`) and the standard exports (`metadata`, `describe`,
`validate`, `calculate_metrics`, `generate_passport`).

## When to use this crate

- You are authoring a Wasm product group plugin and want the standard host/guest ABI
  wiring without hand-rolling alloc/dealloc and JSON packing.
- You need the helper functions that serialise/deserialise the `Plugin*` types
  defined in `dpp-plugin-traits` and want to reuse the same `dpp_rules`
  implementation as the host.

## Example

```rust
use dpp_plugin_sdk::traits::*;
use dpp_plugin_sdk::validate::Validator;

#[derive(Default)]
struct BatteryPlugin;

impl DppProductGroupPlugin for BatteryPlugin {
    fn plugin_identity(&self) -> PluginIdentity {
        todo!("product group key, plugin name and semantic version")
    }
    fn schema_version_range(&self) -> SchemaVersionRange {
        todo!("the schema versions this plugin accepts")
    }
    fn validate_input(&self, input: &PluginInput) -> Result<(), PluginError> {
        // `Validator` collects every failure rather than stopping at the first,
        // so a manufacturer sees the whole form's problems in one response.
        //
        // `require_product_identifier` picks which field to check from the
        // declared EN 18219 clause 5 scheme. Do NOT reach for `require_gtin`
        // here: it reads a flat top-level `gtin`, which product group data does
        // not carry — the GTIN lives inside `productIdentifier`, and only under
        // scheme 1. Schemes 2 and 3 have no GTIN at all.
        Validator::new(input)
            .require_product_identifier("productIdentifier")
            .require_str("batteryChemistry")
            .require_positive("nominalVoltageV")
            .finish()
    }
    fn calculate_metrics(&self, input: &PluginInput) -> Result<PluginResult, PluginError> {
        todo!("compliance determination")
    }
    fn generate_passport(&self, input: PluginInput) -> Result<serde_json::Value, PluginError> {
        todo!("the product group payload")
    }
}

// `meta` and `capabilities` have default implementations — override them only
// if the defaults are wrong for your product group.
//
// Then call the macro exactly once at the crate root to generate the ABI:
//
//     dpp_plugin_sdk::export_plugin!(BatteryPlugin);
```

`plugins/product-group-battery` is the reference implementation — read it before writing a new one.

## Relationship to other crates

| Crate | Role |
|---|---|
| `dpp-plugin-traits` | Defines the host/guest contract (`PluginMeta`, `PluginCapabilities`, `DppProductGroupPlugin`) |
| `dpp-rules` | Re-exported rule implementations so plugins share the same rules engine as the host |

## Minimum Rust version

1.96 (MSRV is enforced in CI)

## License

Apache-2.0 — see [LICENSE](../../LICENSE)
