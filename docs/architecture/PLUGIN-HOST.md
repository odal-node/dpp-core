# Wasm plugin system

Product-group compliance logic runs as Wasm plugins compiled to `wasm32-wasip1`. This
document covers the plugin ABI (defined in `dpp-plugin-traits`), how to build plugins,
and how compliance dispatch works.

---

## 1. Architecture

The plugin system has three layers:

- **`dpp-plugin-traits`:** the types of the host and guest contract: the
  `DppProductGroupPlugin` trait, `PluginCapabilities` (ABI and schema-version
  negotiation), `PluginResult`, `PluginError` and the `AbiResult` envelope.
- **`dpp-plugin-sdk`:** the guest-side SDK plugin authors depend on. A plugin implements
  `DppProductGroupPlugin` and calls `export_plugin!(MyPlugin)` once; the macro generates
  the whole Wasm ABI and wires each export to a trait method.
- **`PluginHost`:** the port trait in `dpp_domain::ports` that a runtime host implements to
  load and run plugins. This library ships no implementation; yours lives in your
  application.

In this document, **the host** means whatever program implements `PluginHost` and runs the
plugins, and **the guest** is the plugin.

> **Dispatch is by product group, never by product category.** The host selects a plugin
> from the `ProductGroup` of the passport data. A product category (battery `portable`
> or `ev`, electronics `smartphone`) is data the plugin may branch on internally; it never
> changes which plugin runs. See [DATA-MODEL.md](DATA-MODEL.md) §3.4.

```text
ComplianceRegistry::compute(product_group, data)
    |
    +-- has plugin? --> PluginHost::compute(product_group, data)
    |                       |
    |                       +-- JSON in through linear memory
    |                       +-- calculate_metrics()
    |                       +-- JSON out through linear memory
    |
    +-- no plugin --> PassthroughNoValidation
```

### Determination path

`ComplianceRegistry` is the **determination port**, the extension point for compliance
logic. Its implementations do different jobs:

| Implementation | Where | Role |
|---|---|---|
| `PassthroughRegistry` | this library | Computes nothing and reports `PassthroughNoValidation` for every product group. The default that needs no infrastructure |
| Plugin-backed registry | a host application | Delegates to `PluginHost` and the Wasm product group plugins. **The standard open determination path** |
| Any other implementation | a host application | For example, calculators from `dpp-calc` wired in by your application |

A computed determination passes through `gate_determination(catalog.is_in_force(product_group), …)`
so that a **provisional** product group can never produce a binding `Compliant` or
`NonCompliant`; it is downgraded to `NotAssessed`.

**Validation is independent of all this.** `validate_product_group_data` (JSON Schema
through the registry plus the cross-field rules in `dpp-rules`) runs in the library with
no Wasm host. An application that disables plugins still gets full structural and
cross-field validation and only gives up a *computed determination*. A determination path
without Wasm is intentionally not offered: it would bring closed per-group logic back into
the library.

### Emitting findings (ABI 1.1)

Besides metrics and status, a plugin's `calculate_metrics` returns two finding lists on
`PluginResult` (ABI 1.1, backward-compatible):

- `violations`: **binding**. The host blocks publish when the product group's passport
  obligation `is_in_force` and `violations` is non-empty.
- `warnings`: **advisory or experimental**. Shown on the determination but never blocking
  (for example, a recycled-content target that is not yet in force).

The plugin produces them by calling the shared rules the SDK re-exports as
`dpp_plugin_sdk::rules` (`dpp-rules`). The host maps each `PluginFinding` to a
`ComplianceFinding` on the core `ComplianceResult` (`plugin_result_to_compliance`), stores
it on the passport (`compliance_result`, part of the signed payload) and gates publish on
it.

`product-group-battery` is the reference. It scopes the Regulation (EU) 2023/1542 Art. 8(2)
recycled-content checks to the declared chemistry and emits them as **advisory** warnings
(the Art. 8(2) minimums are not binding until 18 August 2031). Data-integrity
contradictions, such as cobalt declared on an LFP cell or an inverted operating
temperature range, are hard **validation** errors raised in the core `cross_field_errors`.

**Recipe for adding a determination to a product group:** in that group's plugin
`calculate_metrics`, call the relevant `dpp_plugin_sdk::rules::<product group>` checks, push
`PluginFinding`s onto `warnings` (not yet in force, or advisory) or `violations` (binding
and in force), and rebuild the `.wasm`. No host change is needed: the host routes by
product group, then stores and gates the result.

---

## 2. ABI contract

`export_plugin!` generates these exports over Wasm linear memory. Authors do not write
them by hand.

```rust
extern "C" fn alloc(len: u32) -> u32;          // allocate len bytes, return ptr
extern "C" fn dealloc(ptr: u32, len: u32);     // free a previously allocated slice

extern "C" fn metadata() -> u64;               // -> PluginMeta JSON
extern "C" fn describe() -> u64;               // -> PluginCapabilities JSON
extern "C" fn validate(ptr: u32, len: u32) -> u64;            // -> AbiResult
extern "C" fn calculate_metrics(ptr: u32, len: u32) -> u64;   // -> AbiResult (ok: PluginResult)
extern "C" fn generate_passport(ptr: u32, len: u32) -> u64;   // -> AbiResult (ok: payload)
```

Every `-> u64` packs the output buffer as `(out_ptr << 32) | out_len`. Input and output are
UTF-8 JSON in linear memory: the host writes the input JSON at the pointer `alloc`
returned, calls the entry point, reads the JSON from the returned packed pointer, then
`dealloc`s both buffers.

**Version negotiation.** Right after loading a plugin, the host calls `describe()` to read
its `PluginCapabilities` (declared ABI version, supported schema-version ranges, feature
capabilities) and runs `dpp_plugin_traits::check_compatibility` **before** dispatching any
work. A plugin whose ABI major version or schema range does not match is refused. This is
what makes `VersionedSchemaRegistry` enforceable at the Wasm boundary.

**Fallible calls.** `validate`, `calculate_metrics` and `generate_passport` return an
`AbiResult` envelope (`{ "ok": <value> }` on success, `{ "error": <PluginError> }` on
failure), because a Rust `Result` cannot cross the C ABI. The host deserialises it back
into the typed contract.

WIT interface definitions may replace this low-level ABI as the Wasm Component Model
matures. That is not planned work yet.

---

## 3. Available plugins

All eleven plugins use the SDK (`dpp-plugin-sdk` and `export_plugin!`):

| Plugin | Product group |
|---|---|
| `product-group-battery` | battery (the reference implementation) |
| `product-group-textile` | textile, unsold-goods |
| `product-group-steel` | steel |
| `product-group-electronics`, `-construction`, `-tyre`, `-toy`, `-aluminium`, `-furniture`, `-mattress`, `-detergent` | the group of the same name |

Schema versions are not listed here: several groups are past v1.0.0, and a list went stale
the first time one moved. A plugin validates against whatever versions its product group
has registered in `dpp_domain::schemas::embedded`, which is the authority.

Twelve product groups, eleven plugins: `unsold-goods` has no plugin of its own and runs on
`product-group-textile`, which dispatches on the payload's `productGroup`. `mattress` was
split out of `furniture` because the ESPR working plan makes mattresses a separate product
group. Furniture's plugin cannot serve it (it requires a `productType` a mattress payload
does not carry), so mattress has its own plugin, with furniture's checks minus that field.

---

## 4. Building plugins

```bash
rustup target add wasm32-wasip1
just build-plugins
```

The plugins share one workspace, `plugins/Cargo.toml`, separate from the library
workspace. To build one plugin, run from `plugins/`:

```bash
cargo build -p product-group-battery --target wasm32-wasip1 --release
```

Output: `plugins/target/wasm32-wasip1/release/product_group_battery.wasm`. The release
profile strips symbols and optimises for size; aim for under 2 MB per plugin.

---

## 5. Writing a new plugin

1. Create `plugins/product-group-<name>/` with a copy of
   `plugins/product-group-battery/Cargo.toml` (it inherits version, edition, licence and
   dependencies from the plugin workspace), and add the directory to `members` in
   `plugins/Cargo.toml`.
2. Define a unit struct, implement `DppProductGroupPlugin` (`plugin_identity`,
   `schema_version_range`, `validate_input`, `calculate_metrics`, `generate_passport`),
   and call `export_plugin!(MyPlugin)` once. Do **not** hand-write `alloc`, `dealloc` or
   the other exports. `meta()` and `capabilities()` derive from these; override them only
   if the plugin needs other values.
3. The default `capabilities()` declares the ABI version (`AbiVersion::current()`) and the
   `SchemaVersionRange` values from `schema_version_range()`. The host enforces them
   through `describe()`.
4. For a new product group, add its manifest and first schema under `crates/dpp-domain/`
   as [CONTRIBUTING.md §6](../../CONTRIBUTING.md#6-changing-a-schema) describes.
5. Add unit tests for the trait implementation. They run on the host target with
   `cargo test`.

The plugin receives the product-group data as JSON, validates the regulatory fields, runs
the compliance calculations, and returns a typed `PluginResult`.

---

## 6. Security model

Plugins run in a sandboxed Wasm environment with no access to the host system:

| Capability | Status |
|---|---|
| Filesystem | Denied |
| Network | Denied |
| System random | Denied |
| Threads | Denied (single-threaded) |
| CPU | Capped by fuel metering |
| Memory | Capped per instance |

The host sets the specific fuel and memory limits.

---

## 7. Signed plugins

A `.wasm` file is code a host will run, so a host should verify a detached Ed25519
signature over it, made with a plugin signing key the operator trusts, before compiling it.
This library defines no signature format and does no verification: signing, key
distribution and the refusal policy belong to the host.
