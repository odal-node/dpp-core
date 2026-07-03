# dpp-plugin-traits

[![crates.io](https://img.shields.io/crates/v/dpp-plugin-traits.svg)](https://crates.io/crates/dpp-plugin-traits)
[![docs.rs](https://img.shields.io/docsrs/dpp-plugin-traits)](https://docs.rs/dpp-plugin-traits)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](../../LICENSE)

Host/guest ABI contract for [Odal Node](https://odal-node.io) Wasm sector plugins.

`no_std` compatible. Defines the interface between the platform runtime (the Wasmtime
host in `dpp-engine`) and sector-specific compliance rules compiled to
`wasm32-wasip1`. This crate contains no business logic — only the shared types that
both sides of the boundary agree on.

## When to use this crate

- You are authoring a new sector plugin (a Rust crate compiled to Wasm) and need
  the `DppSectorPlugin` trait.
- You are building a Wasmtime host that loads sector plugins and need the ABI types
  for capability negotiation and field error reporting.

## Example

```rust
use dpp_plugin_traits::{DppSectorPlugin, PluginMeta, PluginFieldError, AbiVersion};

struct BatteryPlugin;

impl DppSectorPlugin for BatteryPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta {
            name: "sector-battery".into(),
            version: "0.1.0".into(),
            sector: "battery".into(),
            abi: AbiVersion::CURRENT,
            capabilities: Default::default(),
            schema_constraint: None,
        }
    }

    fn validate(&self, data: &[u8]) -> Vec<PluginFieldError> {
        // sector-specific validation logic
        vec![]
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
