# dpp-plugin-traits

[![crates.io](https://img.shields.io/crates/v/dpp-plugin-traits.svg)](https://crates.io/crates/dpp-plugin-traits)
[![docs.rs](https://img.shields.io/docsrs/dpp-plugin-traits)](https://docs.rs/dpp-plugin-traits)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](../../LICENSE)

The host and guest contract for Wasm product group plugins in EU Digital Product
Passport systems. Part of [dpp-core](https://github.com/odal-node/dpp-core) by
[Odal Node](https://odal-node.io).

It defines the interface between a host runtime (a wasmtime host, for example) and
product-group-specific compliance rules compiled to `wasm32-wasip1`. It holds no
business logic, only the types both sides of the boundary agree on. It uses `std` types
(`String`, `Vec`, `HashMap`) and is **not** `no_std`.

## When to use this crate

- You write a product group plugin (a Rust crate compiled to Wasm) and need the
  `DppProductGroupPlugin` trait.
- You build a host that loads product group plugins and need the ABI types for
  capability negotiation and field error reporting.

## Example

```rust
use dpp_plugin_traits::{
    DppProductGroupPlugin, PluginError, PluginIdentity, PluginInput, PluginResult, SchemaVersionRange,
};
use serde_json::Value;

struct BatteryPlugin;

impl DppProductGroupPlugin for BatteryPlugin {
    // `meta()` and `capabilities()` are built from these two for you.
    // Override them directly only if a plugin needs different values.
    fn plugin_identity(&self) -> PluginIdentity {
        PluginIdentity {
            product_group: "battery",
            name: "Odal Node Battery Plugin",
            version: env!("CARGO_PKG_VERSION"),
            description: "EU Battery Regulation 2023/1542 structural validation",
        }
    }

    fn schema_version_range(&self) -> SchemaVersionRange {
        SchemaVersionRange {
            min_version: "1.0.0".into(),
            max_version: "2.0.0".into(),
        }
    }

    fn validate_input(&self, input: &PluginInput) -> Result<(), PluginError> {
        // product-group-specific validation logic
        Ok(())
    }

    fn calculate_metrics(&self, input: &PluginInput) -> Result<PluginResult, PluginError> {
        // product-group-specific compliance metrics
        todo!()
    }

    fn generate_passport(&self, input: PluginInput) -> Result<Value, PluginError> {
        Ok(input)
    }
}
```

## Relationship to other crates

This crate has **no workspace dependencies**, so plugin authors get a small, stable
dependency. `dpp-plugin-sdk` builds the guest-side ABI on top of it.

## Minimum Rust version

1.96, enforced in CI.

## Licence

Apache-2.0. See [LICENSE](../../LICENSE).
