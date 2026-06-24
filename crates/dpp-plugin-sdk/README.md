# dpp-plugin-sdk

Guest-side SDK and ABI export macro for Odal Node Wasm sector plugins. This crate
provides the glue and helper utilities that make writing a Wasm sector plugin
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
use dpp_plugin_sdk::{export_plugin, traits::*};

#[derive(Default)]
struct BatteryPlugin;

impl DppSectorPlugin for BatteryPlugin {
    // implement `meta`, `capabilities`, `validate_input`, `calculate_metrics`, `generate_passport`
}

export_plugin!(BatteryPlugin);
```

## Relationship to other crates

| Crate | Role |
|---|---|
| `dpp-plugin-traits` | Defines the host/guest contract (`PluginMeta`, `PluginCapabilities`, `DppSectorPlugin`) |
| `dpp-rules` | Re-exported rule implementations so plugins share the same rules engine as the host |

## Minimum Rust version

1.96 (MSRV is enforced in CI)

## License

Apache-2.0 — see [LICENSE](../../LICENSE)
