# Odal Node Core — Architecture

## Overview

Odal Node Core is a pure Rust library for EU ESPR Digital Product Passport compliance. It defines domain types, cryptographic primitives, schema validation, and port traits — the complete standard for what a DPP is, how it is signed, and how compliance is verified.

No HTTP framework. No database. No async runtime (except where port traits require it for downstream implementors). The entire workspace compiles with nothing running.

---

## Module Architecture

```
+--------------------------------------------------+
|                  dpp-domain                       |
|  Domain types, port traits, schema validation     |
|  VersionedSchemaRegistry, SectorCatalog,          |
|  ComplianceRegistry                               |
+--------------------------------------------------+
        ^            ^             ^
        |            |             | depends on
+-------------------+ |   +---------------------+
|    dpp-crypto     | |   |     dpp-registry    |
|  Ed25519, JWS,    | |   |  EU registry types  |
|  DID builder,     | |   |    (wasm32-safe)    |
| LocalIdentitySvc  | |   +---------------------+
+-------------------+ |
                      v
            +-------------------+
            |     dpp-rules     |   (no_std, zero-dep
            | cross-field rules |    cross-field rules;
            +-------------------+    dpp-domain depends on it)
                      ^
                      | re-exported by
+------------------+  |  +------------------+   +---------------------+
| dpp-plugin-traits|  +--|  dpp-plugin-sdk  |   | dpp-digital-link    |
|     Wasm ABI     |-----| export_plugin!   |   | GS1 Digital Link    |
+------------------+     +------------------+   +---------------------+

+---------------------+
|      dpp-calc       |  EU-methodology calculators (CO2e,
| pure, FactorProvider|  repairability); receipts; pure
+---------------------+

dpp-tests — cross-crate integration tests (not published)
```

---

## dpp-domain — The Domain

The dependency root. Every other crate may depend on it; it depends on nothing internal.

### Domain Types

Canonical DPP types: `Passport`, `PassportId`, `ProductCategory`, `ManufacturerInfo`, `MaterialEntry`, `BatteryData`, `TextileData`, `SignedCredential`. All types derive `Serialize` + `Deserialize` and are `wasm32`-safe.

### Port Traits

Trait definitions that downstream projects implement against their own infrastructure:

| Trait | Async | Purpose |
|---|---|---|
| `PassportRepository` | yes | CRUD for DPP records |
| `ComplianceRegistry` | no | Route sector data to the correct compliance strategy |
| `ComplianceStrategy` | no | Validate sector-specific compliance rules |
| `IdentityPort` | yes | Sign and verify passport JWS |
| `PluginHost` | no | Dispatch to Wasm sector plugins |
| `ArchivePort` | yes | Immutable DPP archival with retention guarantees |
| `RegistrySyncPort` | yes | EU Central Registry registration and status sync |
| `SealPort` | yes | eIDAS qualified electronic seal (ESPR Art. 13 / eIDAS 910/2014) |

The async traits use `async-trait`. The sync traits are plain Rust traits — compatible with `no_std` and `wasm32`.

`dpp-crypto` provides `LocalIdentityService`, a concrete `IdentityPort` implementation backed by the local `KeyStore`.

### VersionedSchemaRegistry

Embeds all JSON schemas from `crates/dpp-domain/schemas/{sector}/v{version}.json` via `include_str!()` (inside the crate so they publish with it). Provides:

- `get(sector, version)` — retrieve a specific schema
- `latest(sector)` — retrieve the newest version for a sector
- `validate(sector, version, data)` — validate passport data against a schema
- `list()` — enumerate all available (sector, version) pairs

Schema validation is gated behind `#[cfg(not(target_arch = "wasm32"))]` because the `jsonschema` crate is not wasm32-compatible.

---

## dpp-crypto — Cryptographic Primitives

Pure signing and key management. No HTTP, no database.

### KeyStore

AES-256-GCM encrypted Ed25519 key storage. Keys are persisted as JSON files on the local filesystem. The path is injected, making it testable with temp directories.

- `open(path, passphrase)` — open or create a key store
- `generate_key(key_id)` — generate a new Ed25519 keypair
- `load_key(key_id)` — load an existing key
- `archive_key(key_id)` — archive current key (for rotation)
- `load_archived_keys(key_id)` — load all archived keys

### Signer

JWS compact signing (EdDSA with Ed25519):

- `sign(store, key_id, payload)` — produce a JWS compact serialisation
- `verify(jws, public_key)` — verify a JWS signature

### DID Builder

Constructs `did:web` DID documents from the KeyStore state:

- `build_did_document(store, base_url, key_id)` — builds the full DID document with the current primary key as `#key-1` (authentication) and archived keys as `#key-2`, `#key-3`, etc. (assertionMethod)

### JWS Verifier

Single source of truth for JWS verification:

- `verify_jws(jws, public_key_b64)` — verify a JWS against a base64-encoded public key
- `extract_primary_public_key(did_document)` — extract the primary Ed25519 public key from a DID document

### LocalIdentityService

Concrete implementation of `dpp-domain::ports::IdentityPort` backed by the local `KeyStore`. Wires together the signer, DID builder, and JWS verifier into the port trait interface:

- `sign_passport(passport_id, payload)` — signs the payload with the issuer's Ed25519 key, builds the DID document, and returns a `SignedCredential`
- `verify_signature(jws, payload)` — resolves the issuer's DID document from the `KeyStore` and verifies the JWS

---

## dpp-registry — EU Registry Interface

Data types for the EU Central DPP Registry (ESPR Article 13). `wasm32`-safe — no I/O, no HTTP, no async runtime. Contains only wire-format types: `RegistrationPayload`, `EuRegistryEnvelope`, `EuRegistryResponse`, `StatusResponse`, `TransferNotification`, identifier structs (`ProductIdentifier`, `OperatorIdentifier`, `FacilityIdentifier`, `ProductItemIdentifier`), error types, and `RegistryEndpoint` with sandbox/production presets.

The port trait (`RegistrySyncPort`) and its ghost implementation (`GhostRegistrySync`) live in `dpp-domain::ports::registry_sync`, not here. The platform adapter (`EuRegistrySync`) in `dpp-engine/dpp-node/src/infra/` implements the port using `reqwest` and these bridge types.

---

## dpp-digital-link — GS1 Digital Link, AAS, JSON-LD, Link-Type Negotiation

Pure, stateless crate — no I/O or network dependencies. Compiles to both `std` and `wasm32`. Four submodules:

- `digital_link` — GS1 Digital Link URI parsing and building (`DigitalLink::parse`/`build`), the GTIN/serial/batch application-identifier table, and QR URL construction.
- `aas` — maps a `Passport` to IDTA Asset Administration Shell shells and submodels (`build_aas_from_passport`), with a dedicated submodel builder per sector plus the sector-agnostic core submodels (identification, manufacturer, environmental, materials, repairability).
- `jsonld` — wraps/strips a passport payload in a minimal JSON-LD `@context` envelope for GS1/Schema.org/EU ESPR semantic interoperability.
- `linktype` — GS1 link-type vocabulary and content-negotiation (`negotiate`) between a client's `Accept` header and a passport's available link descriptors.

---

## dpp-plugin-traits — Wasm Plugin ABI

Types for the host/guest contract. Uses `std` types (`String`, `Vec`, `HashMap`) — not `no_std`. Sector plugins compiled to `wasm32-wasip1` implement this ABI (generated by `dpp-plugin-sdk`'s `export_plugin!` macro — authors do not hand-write it):

- `alloc(len: u32) -> u32` — allocate `len` bytes, return ptr
- `dealloc(ptr: u32, len: u32)` — matching dealloc
- `metadata() -> u64` — returns `PluginMeta` JSON
- `describe() -> u64` — returns `PluginCapabilities` JSON (host runs `check_compatibility` before dispatch)
- `validate(ptr, len) -> u64` — returns the `AbiResult` envelope
- `calculate_metrics(ptr, len) -> u64` — returns `AbiResult` (`ok: PluginResult`)
- `generate_passport(ptr, len) -> u64` — returns `AbiResult` (`ok: payload`)

Each `-> u64` packs the output as `(out_ptr << 32) | out_len`. Input/output is UTF-8 JSON over Wasm linear memory.

---

## Proof-Bound Architecture

Odal never stores raw production data. The library validates product data against the sector schema, signs it with the manufacturer's Ed25519 key, and produces a cryptographically verifiable proof. The raw data is the manufacturer's responsibility. The signed proof is what gets persisted and served.

This satisfies GDPR data minimisation and the EU ESPR trust architecture.

---

## Wasm Targets

Two wasm32 targets are supported:

| Target | Crates | Purpose |
|---|---|---|
| `wasm32-unknown-unknown` | dpp-registry, dpp-digital-link | Browser/Cloudflare Workers (JS-hosted) |
| `wasm32-wasip1` | sector plugins | wasmtime sandbox (WASI P1 syscall interface) |

`getrandom` uses the JS backend for `wasm32-unknown-unknown` (configured in `.cargo/config.toml`).
