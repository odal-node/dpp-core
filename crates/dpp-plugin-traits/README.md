# dpp-plugin-traits

[![crates.io](https://img.shields.io/crates/v/dpp-plugin-traits.svg)](https://crates.io/crates/dpp-plugin-traits)
[![docs.rs](https://img.shields.io/docsrs/dpp-plugin-traits)](https://docs.rs/dpp-plugin-traits)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](../../LICENSE)

Host/guest ABI contract for [Odal Node](https://odal-node.io) Wasm sector plugins.

Uses `std` types (`String`, `Vec`, `HashMap`) — **not** `no_std`. Defines the
interface between the platform runtime (the Wasmtime host in `dpp-engine`) and
sector-specific compliance rules compiled to `wasm32-wasip1`. This crate
contains no business logic — only the shared types that both sides of the
boundary agree on.

## When to use this crate

- You are authoring a new sector plugin (a Rust crate compiled to Wasm) and need
  the `DppSectorPlugin` trait.
- You are building a Wasmtime host that loads sector plugins and need the ABI types
  for capability negotiation and field error reporting.

## Example

```rust
use dpp_plugin_traits::{
    DppSectorPlugin, PluginError, PluginIdentity, PluginInput, PluginResult, SchemaVersionRange,
};
use serde_json::Value;

struct BatteryPlugin;

impl DppSectorPlugin for BatteryPlugin {
    // `meta()` and `capabilities()` are built from these two for you —
    // override them directly only if a plugin needs different values.
    fn plugin_identity(&self) -> PluginIdentity {
        PluginIdentity {
            sector: "battery",
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
        // sector-specific validation logic
        Ok(())
    }

    fn calculate_metrics(&self, input: &PluginInput) -> Result<PluginResult, PluginError> {
        // sector-specific compliance metrics
        todo!()
    }

    fn generate_passport(&self, input: PluginInput) -> Result<Value, PluginError> {
        Ok(input)
    }
}
```

## Relationship to other crates

This crate has **no workspace dependencies** — it is self-contained by design so
that plugin authors have a minimal, stable dependency.

## Minimum Rust version

1.96 (MSRV is enforced in CI)

## License

Apache-2.0 — see [LICENSE](../../LICENSE)
