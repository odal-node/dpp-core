# dpp-plugin-sdk

[![crates.io](https://img.shields.io/crates/v/dpp-plugin-sdk.svg)](https://crates.io/crates/dpp-plugin-sdk)
[![docs.rs](https://img.shields.io/docsrs/dpp-plugin-sdk)](https://docs.rs/dpp-plugin-sdk)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](../../LICENSE)

Guest-side SDK and ABI export macro for Wasm product group plugins. Part of
[dpp-core](https://github.com/odal-node/dpp-core) by [Odal Node](https://odal-node.io).

One `export_plugin!` call generates the linear-memory ABI (`alloc`, `dealloc`) and the
standard exports (`metadata`, `describe`, `validate`, `calculate_metrics`,
`generate_passport`). The crate also provides a field `Validator` and re-exports the
`dpp-rules` implementations, so a plugin applies the same rules as the host.

## When to use this crate

- You write a Wasm product group plugin and want the standard host and guest wiring
  without hand-written `alloc`/`dealloc` and JSON packing.
- You need helpers that serialise and deserialise the `Plugin*` types from
  `dpp-plugin-traits`, and the same `dpp_rules` implementations the host uses.

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
        // declared EN 18219 clause 5 scheme. Do NOT use `require_gtin` here: it
        // reads a flat top-level `gtin`, which product group data does not
        // carry. The GTIN lives inside `productIdentifier`, and only under
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

// `meta` and `capabilities` have default implementations. Override them only
// if the defaults are wrong for your product group.
//
// Then call the macro exactly once at the crate root to generate the ABI:
//
//     dpp_plugin_sdk::export_plugin!(BatteryPlugin);
```

`require_gtin` is still correct for a bare GS1 field, and `require_product_identifier`
uses it for the scheme 1 branch. An unrecognised scheme is refused rather than skipped,
because an unmapped scheme string is where an invented identifier would otherwise pass
unexamined.

`plugins/product-group-battery` is the reference implementation. Read it before you
write a new plugin.

## Relationship to other crates

| Crate | Role |
|---|---|
| `dpp-plugin-traits` | Defines the host and guest contract (`PluginMeta`, `PluginCapabilities`, `DppProductGroupPlugin`) |
| `dpp-rules` | Re-exported rule implementations, so plugins share the host's rules |

## Minimum Rust version

1.96, enforced in CI.

## Licence

Apache-2.0. See [LICENSE](../../LICENSE).
