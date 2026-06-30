# Wasm Plugin System

Sector compliance validation is implemented as Wasm plugins compiled to `wasm32-wasip1`. This document covers the plugin ABI (defined in `dpp-plugin-traits`), how to build plugins, and how the compliance dispatch model works.

---

## 1. Architecture

The plugin system has three layers:

**dpp-plugin-traits** (dpp-core) — types defining the host/guest contract: the `DppSectorPlugin` trait, `PluginCapabilities` (ABI + schema-version negotiation), `PluginResult`, `PluginError`, and the `AbiResult` envelope.

**dpp-plugin-sdk** (dpp-core) — the guest-side SDK plugin authors depend on. A plugin implements `DppSectorPlugin` and calls `export_plugin!(MyPlugin)` once; the macro generates the entire Wasm ABI and wires each export to a trait method. Plugins no longer hand-roll the ABI or redefine output structs.

**PluginHost** (port trait in `dpp-domain::ports`) — the trait that a runtime host implements to load and execute plugins. The host implementation lives downstream.

> **Dispatch is by sector, never by product category.** The host selects a plugin from the `Sector` of the passport data. A product category (e.g. battery `portable` vs `ev`, electronics `smartphone`) is *sector data* the plugin may branch on internally — it never changes which plugin runs. See `DATA-MODEL.md` §3.5.

```
ComplianceRegistry::compute(sector, data)
    |
    +-- has plugin? --> PluginHost::compute(sector, data)
    |                       |
    |                       +-- JSON in via linear memory
    |                       +-- calculate_metrics()
    |                       +-- JSON out via linear memory
    |
    +-- no plugin --> PassthroughNoValidation
```

### Determination path

`ComplianceRegistry` is the **determination port** (the open-core extension seam). It has several implementations, which are *not* redundant:

| Implementation | Where | Role |
|---|---|---|
| `PassthroughRegistry` | dpp-core (Apache) | Computes nothing — `PassthroughNoValidation` for every sector. The no-infra default. |
| Plugin-backed registry | platform (Apache) | Delegates to `PluginHost` → the Wasm sector plugins. **The canonical OSS determination path.** |
| `PremiumComplianceRegistry` | proprietary | Paid calculators. |

A computed determination is passed through `gate_determination(catalog.is_in_force(sector), …)` (dpp-core) so a **provisional** sector can never surface a binding `Compliant`/`NonCompliant` — it is downgraded to `NotAssessed`.

**Validation is independent of all of this.** `validate_sector_data` (JSON Schema via the registry + cross-field `dpp-rules`) runs in pure core with no Wasm host. A self-hoster who disables plugins still gets full structural and cross-field validation; they only forgo a *computed determination*. A no-Wasm determination path is intentionally not offered (it would re-introduce closed per-sector logic into core). See `SECTOR-MODEL-CONSOLIDATION.md` §3.1.

### Emitting findings (ABI 1.1)

A plugin's `calculate_metrics` returns, in addition to metrics + status, two
finding lists on `PluginResult` (ABI 1.1, backward-compatible):

- `violations` — **binding**. The host blocks publish when the sector's DPP
  obligation `is_in_force` and `violations` is non-empty.
- `warnings` — **advisory / experimental**. Surfaced on the determination but
  never block (e.g. a recycled-content target that is not yet in force).

The plugin produces these by calling the shared regulatory rules the SDK
re-exports as `dpp_plugin_sdk::rules` (`dpp-rules`). The host maps each
`PluginFinding` to a `ComplianceFinding` on the core `ComplianceResult`
(`plugin_result_to_compliance`), which the engine persists on the passport
(`compliance_result`, part of the signed payload) and gates publish on.

`sector-battery` is the reference: it scopes EU 2023/1542 Annex X recycled-content
checks to the declared chemistry and emits them as **advisory** warnings (the
Phase-1 minima are not binding until 18 Aug 2031), while data-integrity
contradictions (cobalt declared on an LFP cell; inverted operating-temperature
range) are hard **validation** errors raised in core `cross_field_errors`.

**Recipe — add determination to a sector:** in that sector's plugin
`calculate_metrics`, call the relevant `dpp_plugin_sdk::rules::<sector>` checks,
push `PluginFinding`s onto `warnings` (not-yet-in-force / advisory) or
`violations` (binding, in force), rebuild the `.wasm`. No engine change is
required — the host routes by sector and the engine persists + gates the result.

---

## 2. ABI Contract

`export_plugin!` generates the following exports over Wasm linear memory. Authors do not write these by hand.

```rust
extern "C" fn alloc(len: u32) -> u32;          // allocate len bytes, return ptr
extern "C" fn dealloc(ptr: u32, len: u32);     // free a previously allocated slice

extern "C" fn metadata() -> u64;               // -> PluginMeta JSON
extern "C" fn describe() -> u64;               // -> PluginCapabilities JSON
extern "C" fn validate(ptr: u32, len: u32) -> u64;            // -> AbiResult
extern "C" fn calculate_metrics(ptr: u32, len: u32) -> u64;   // -> AbiResult (ok: PluginResult)
extern "C" fn generate_passport(ptr: u32, len: u32) -> u64;   // -> AbiResult (ok: payload)
```

Every `-> u64` packs the output buffer as `(out_ptr << 32) | out_len`. Input/output is UTF-8 JSON over linear memory: the host writes input JSON at the pointer from `alloc`, calls the entry point, reads the JSON from the returned packed pointer, then `dealloc`s the buffers.

**Version negotiation.** Immediately after loading a plugin the host calls `describe()` to read its `PluginCapabilities` (declared ABI version, supported schema-version ranges, feature capabilities) and runs `dpp_plugin_traits::check_compatibility` **before** dispatching any work. A plugin whose ABI major version or schema range does not match is refused. This is what makes the `VersionedSchemaRegistry` enforceable at the Wasm boundary rather than aspirational.

**Fallible calls.** `validate`, `calculate_metrics`, and `generate_passport` return an `AbiResult` envelope — `{ "ok": <value> }` on success or `{ "error": <PluginError> }` on failure — since a Rust `Result` cannot cross the C ABI. The host deserialises this back into the typed contract.

Future: WIT interface definitions will replace this low-level ABI as the Wasm Component Model matures.

---

## 3. Available Plugins

All ten plugins run on the SDK (`dpp-plugin-sdk` + `export_plugin!`):

| Plugin | Sector | Schema | ABI |
|---|---|---|---|
| `sector-battery` | battery | `schemas/battery/v{1.0.0,2.0.0}.json` | SDK (`DppSectorPlugin`) |
| `sector-textile` | textile, unsold-goods | `schemas/textile/*`, `unsold-goods/*` | SDK (`DppSectorPlugin`) |
| `sector-steel` | steel | `schemas/steel/v1.0.0.json` | SDK (`DppSectorPlugin`) |
| `sector-electronics`, `-construction`, `-tyre`, `-toy`, `-aluminium`, `-furniture`, `-detergent` | resp. | `schemas/{sector}/v1.0.0.json` | SDK (`DppSectorPlugin`) |

Plugins are standalone Rust crates excluded from the workspace. Each depends on `dpp-plugin-sdk` (which re-exports `dpp-plugin-traits`), implements `DppSectorPlugin`, and calls `export_plugin!` once — none hand-roll the ABI. **`sector-battery` is the reference implementation.**

---

## 4. Building Plugins

```bash
rustup target add wasm32-wasip1
cargo build -p sector-textile --target wasm32-wasip1 --release
```

Output: `target/wasm32-wasip1/release/sector_textile.wasm`

Target size: < 2 MB per plugin. Strip is enabled via `profile.release.strip = true`.

The `just build-plugins` recipe builds all sector plugins in one command.

---

## 5. Writing a New Plugin

1. Create a new crate in `plugins/sector-{name}/` (`crate-type = ["cdylib"]`, empty `[workspace]` to detach).
2. Add `dpp-plugin-sdk = { path = "../../crates/dpp-plugin-sdk" }`.
3. Define a unit struct, implement `DppSectorPlugin` (`meta`, `capabilities`, `validate_input`, `calculate_metrics`, `generate_passport`), and call `export_plugin!(MyPlugin)` once. Do **not** hand-write `alloc`/`dealloc`/etc.
4. In `capabilities()`, declare the ABI version (`AbiVersion::current()`) and the `SchemaVersionRange`(s) the plugin supports — the host enforces these via `describe()`.
5. Add the JSON schema at `schemas/{sector}/v1.0.0.json`; the `VersionedSchemaRegistry` picks it up automatically.
6. Add unit tests for the trait impl (they run on the host target with `cargo test`).

`sector-battery` is the canonical reference. The plugin receives sector-specific passport data as JSON, validates regulatory fields, performs compliance calculations, and returns a typed `PluginResult`.

---

## 6. Security Model

Plugins run in a sandboxed Wasm environment with no access to the host system:

| Capability | Status |
|---|---|
| Filesystem | DENIED |
| Network | DENIED |
| System random | DENIED |
| Threads | DENIED (single-threaded) |
| CPU | Capped via fuel metering |
| Memory | Capped per instance |

The specific fuel and memory limits are configured by the host implementation.

---

## 7. Signature Verification (Roadmap)

Each `.wasm` file should be accompanied by a `.wasm.sig` Ed25519 signature file, created with a plugin signing key. The host loader will verify the signature before compilation. This ensures only trusted plugins are loaded in production.
