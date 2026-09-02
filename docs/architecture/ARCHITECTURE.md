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
|  VersionedSchemaRegistry, ProductGroupCatalog,          |
|  ComplianceRegistry                               |
+--------------------------------------------------+
        ^            ^             ^
        |            |             | depends on
+-------------------+ |   +---------------------+
|    dpp-crypto     | |   |     dpp-registry    |
|  Ed25519, JWS,    | |   |  EU registry types  |
|  keystore         | |   |    (wasm32-safe)    |
| (no workspace dep)| |   +---------------------+
+-------------------+ |
        ^             |
        | depends on  |
+-------------------+ |
|      dpp-vc       | |  W3C VCs, did:web,
|  credentials,     | |  status lists,
|  LocalIdentitySvc | |  JSON-LD context
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
                                                |      dpp-aas        |
                                                | AAS shells/submodels|
                                                +---------------------+

+---------------------+
|      dpp-calc       |  EU-methodology determinations (Art. 8 recycled
| pure, FactorProvider|  content, CO2e, repairability); ruleset identity,
+---------------------+  effective periods, receipts; pure.
          |              Depends on dpp-rules for thresholds — never the
          +-> dpp-rules  reverse, since the plugins reach dpp-rules too.

dpp-tests — cross-crate integration tests (not published)
```

---

## dpp-domain — The Domain

The dependency root. Every other crate may depend on it; it depends on nothing
internal but `dpp-rules`. Six of them do not — see [VERSIONING.md](../governance/VERSIONING.md).

### Top-level concerns

This is the largest crate in the workspace and the one most able to absorb a new
capability without anyone noticing, so what it contains is inventoried rather
than described. A module added here is a deliberate act: `domain_concerns.rs`
compares this list against `lib.rs` and fails the build in either direction.

The **tier** column is the scope law in [CODE-LAYOUT.md](CODE-LAYOUT.md) §1:
imports may only point up the eight-tier ladder, and two tripwires hold them to
it — one for direction, one for cycles. The table below is ordered by tier, and
that order is a measurement of the import graph rather than a judgement about
what each module does.

| Concern | Tier | What it holds |
|---|---|---|
| `identifier` | 1 | GS1 and customs-classification vocabulary — `Gtin`, `Gln`, `CommodityCode`, `CnCategory`, and the shared mod-10 check digit. Depends on nothing in this crate |
| `catalog` | 3 | `ProductGroupCatalog` — identity, scope, schema versions, disclosure classes, plugin binding. It carries **no law** |
| `credential` | 2 | The W3C Verifiable Credential 2.0 envelope binding a passport to its signed payload |
| `compliance` | 2 | The determination value objects: `ComplianceResult`, its findings, status and error |
| `disclosure` | 2 | The Art. 77(2) lattice — `Audience`, `Disclosure`, and the per-field classification. Tier 2 rather than 3: `Passport::redact` takes an `Audience`, so the vocabulary cannot sit with the policy that filters by it |
| `eol` | 5 | End-of-life declarations, and the derogation a destruction claim must cite |
| `field_error` | 2 | `FieldError` and `ValidationErrors` — the per-field detail a validation failure carries. Split from `error` because the two sit at different levels, and holding them together was a cycle |
| `facility` | 2 | `FacilitySnapshot` — where a product was made, as recorded at issuance |
| `graph` | 5 | The bill-of-materials graph a passport sits in |
| `instrument` | 3 | The legal acts, their `PassportObligation`, and one `InstrumentBinding` per (act, product group) pair. **This is where the law lives** |
| `manufacturer` | 2 | `ManufacturerInfo` — the economic operator that placed the product on the market |
| `material` | 2 | `MaterialEntry` — one declared constituent material |
| `passport` | 4 | The aggregate root, its id, reference and audience-filtered view |
| `product` | 5 | `ProductIdentity` — what identifies a product, independent of its passport |
| `product_group` | 4 | The typed per-group payloads and the `ProductGroupData` union |
| `seal` | 2 | eIDAS seal value objects — format, mode, envelope, verification |
| `status` | 2 | `PassportStatus`, the lifecycle state machine |
| `transfer` | 5 | Transfer of responsibility between operators |
| `access` | 6 | The per-field disclosure contract — `ProductGroupAccessPolicy`, `filter_by_audience` |
| `lint` | 6 | Non-binding plausibility findings. Never gates publish |
| `schemas` | 3 | `VersionedSchemaRegistry`, the embedded JSON Schemas, and the version lenses |
| `validation` | 6 | Schema conformance and cross-field rules — the pass `Passport::validate` deliberately does not run |
| `ports` | 7 | The core↔platform trait boundary (see [PORTS.md](PORTS.md)). **Nothing may import it** |
| `passthrough` | 8 | The Apache-2.0 default `ComplianceRegistry` and its per-group strategies — an adapter, wired by the open-source binary |
| `error` | 4 | `DppError`, the one error every fallible entry point returns. Sits at the level of the deepest thing it wraps — a lens error from `schemas` |

Prefer naming the concerns over asserting a count — a count is the part that
goes stale while every claim around it stays checkable.

> **There was a `domain` module here until 2026-08-26.** It held 52% of the crate
> and its name said nothing, so nothing in it had been *chosen* to be there. It
> was dissolved and its children given real homes, which is most of why this
> table is long: the concerns were always this many, they were just not visible.

<!-- DOMAIN-CONCERNS:BEGIN (one module name per line; parsed by domain_concerns.rs) -->
```
access
catalog
compliance
eol
error
field_error
facility
graph
identifier
credential
disclosure
instrument
lint
manufacturer
material
passport
passthrough
ports
product
product_group
schemas
seal
status
transfer
validation
```
<!-- DOMAIN-CONCERNS:END -->

### Domain Types

Canonical DPP types: `Passport`, `PassportId`, `ManufacturerInfo`, `MaterialEntry`, `BatteryData`, `TextileData`, `SignedCredential`. All types derive `Serialize` + `Deserialize` and are `wasm32`-safe.

### Port Traits

Trait definitions that downstream projects implement against their own infrastructure:

| Trait | Async | Purpose |
|---|---|---|
| `PassportRepository` | yes | CRUD for DPP records |
| `ComplianceRegistry` | no | Route product group data to the correct compliance strategy |
| `ComplianceStrategy` | no | Validate product-group-specific compliance rules |
| `IdentityPort` | yes | Sign and verify passport JWS |
| `PluginHost` | no | Dispatch to Wasm product group plugins |
| `ArchivePort` | yes | Immutable DPP archival with retention guarantees |
| `RegistrySyncPort` | yes | EU Central Registry registration and status sync |
| `SealPort` | yes | eIDAS qualified electronic seal (ESPR Art. 13 / eIDAS 910/2014) |

The async traits use `async-trait`. The sync traits are plain Rust traits — compatible with `no_std` and `wasm32`.

`dpp-vc` provides `LocalIdentityService`, a concrete `IdentityPort` implementation backed by `dpp-crypto`'s local `KeyStore`.

### VersionedSchemaRegistry

Embeds all JSON schemas from `crates/dpp-domain/schemas/{product-group}/v{version}.json` via `include_str!()` (inside the crate so they publish with it). Provides:

- `get(product group, version)` — retrieve a specific schema
- `latest(product group)` — retrieve the newest version for a product group
- `validate(product group, version, data)` — validate passport data against a schema
- `list()` — enumerate all available (product group, version) pairs

Schema validation is gated behind `#[cfg(not(target_arch = "wasm32"))]` because the `jsonschema` crate is not wasm32-compatible.

---

## dpp-crypto — Cryptographic Primitives

Pure signing and key management. No HTTP, no database, and **no workspace
dependencies** — the credential and DID layer that used to sit here is
[`dpp-vc`](#dpp-vc--trust), and the per-field disclosure policy is
`dpp_domain::access`. Signing bytes, deciding whose signature means what, and
deciding which fields a role may see are three jobs, and this crate does the
first.

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

### JWS Verifier

Single source of truth for JWS verification:

- `verify_jws(jws, public_key_b64)` — verify a JWS against a base64-encoded public key
- `extract_primary_public_key(did_document)` — extract the primary Ed25519 public key from a DID document

## dpp-registry — EU Registry Interface

Data types for the EU Central DPP Registry (ESPR Article 13). `wasm32`-safe — no I/O, no HTTP, no async runtime. Contains only wire-format types: `RegistrationPayload`, `EuRegistryEnvelope`, `EuRegistryResponse`, `StatusResponse`, `TransferNotification`, identifier structs (`ProductIdentifier`, `OperatorIdentifier`, `FacilityIdentifier`, `ProductItemIdentifier`), error types, and `RegistryEndpoint` with sandbox/production presets.

The port trait (`RegistrySyncPort`) and its ghost implementation (`GhostRegistrySync`) live in `dpp-domain::ports::registry_sync`, not here. The platform adapter (`EuRegistrySync`) implements the port using `reqwest` and these bridge types.

---

<a id="dpp-vc--trust"></a>
## dpp-vc — Trust

Who may read a passport, and how that is proven. W3C Verifiable Credentials,
`did:web` documents, Bitstring Status List revocation, `LocalIdentityService`
(the `IdentityPort` implementation), and the JSON-LD context those are expressed
in. Depends on `dpp-crypto` for JWS and the keystore.

A credential establishes *which* `Audience` a caller holds; `dpp_domain::access`
maps that audience to fields. Two questions, two crates.

Not a `wasm32-unknown-unknown` target — it inherits `dpp-crypto`'s need for a
platform entropy source.

### DID Builder

Constructs `did:web` DID documents from the KeyStore state:

- `build_did_document(store, base_url, key_id)` — builds the full DID document with the current primary key as `#key-1` (authentication) and archived keys as `#key-2`, `#key-3`, etc. (assertionMethod)

### LocalIdentityService

Concrete implementation of `dpp-domain::ports::IdentityPort` backed by the local `KeyStore`. Wires together the signer, DID builder, and JWS verifier into the port trait interface:

- `sign_passport(passport_id, payload)` — signs the payload with the issuer's Ed25519 key, builds the DID document, and returns a `SignedCredential`
- `verify_signature(jws, payload)` — resolves the issuer's DID document from the `KeyStore` and verifies the JWS


---

## dpp-digital-link — GS1

Pure, stateless — no I/O or network dependencies. Compiles to `std` and
`wasm32`. Two submodules:

- `digital_link` — GS1 Digital Link URI parsing and building (`DigitalLink::parse`/`build`), the GTIN/serial/batch application-identifier table, and QR URL construction.
- `linktype` — GS1 link-type vocabulary and content-negotiation (`negotiate`) between a client's `Accept` header and a passport's available link descriptors.

---

## dpp-aas — Asset Administration Shell

Maps a `Passport` to AAS shells and submodels (`build_aas_from_passport`) —
the product group-agnostic core submodels (identification, manufacturer,
environmental, materials, repairability) plus one product group submodel.

**Masked before mapping.** The builder takes an `Audience` and filters the
passport through the same disclosure seam the public view uses, at the entry
point, before any mapper runs. There is deliberately no unmasked entry point: a
filter applied *after* mapping would match on `idShort`, so a mapper that
misspelled one would emit a restricted field the filter could not recognise.

**Typed mappers only where an act is in force** — `battery`, `electronics`,
`unsold_goods`, plus `textile` as a named carve-out. Every other product group
renders through the generic product group projection, because a hand-written mapper
asserts an AAS submodel template that no standards body has ratified. A CI gate
asserts this directly: a provisional product group gaining a typed mapper fails the
build.

**AAS-shaped output carrying our own semantics.** Every emitted `semanticId` is
either `urn:odal-node:*` or carries a provenance record naming who verified it
against the authority's published source — enforced by a test. The crate
currently emits no third-party identifiers, so no IDTA conformance is claimed.

---

## dpp-plugin-traits — Wasm Plugin ABI

Types for the host/guest contract. Uses `std` types (`String`, `Vec`, `HashMap`) — not `no_std`. Product group plugins compiled to `wasm32-wasip1` implement this ABI (generated by `dpp-plugin-sdk`'s `export_plugin!` macro — authors do not hand-write it):

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

Odal never stores raw production data. The library validates product data against the product group schema, signs it with the manufacturer's Ed25519 key, and produces a cryptographically verifiable proof. The raw data is the manufacturer's responsibility. The signed proof is what gets persisted and served.

This satisfies GDPR data minimisation and the EU ESPR trust architecture.

---

## Wasm Targets

Two wasm32 targets are supported:

| Target | Crates | Purpose |
|---|---|---|
| `wasm32-unknown-unknown` | dpp-registry, dpp-digital-link, dpp-aas | Browser/Cloudflare Workers (JS-hosted). Not `dpp-crypto`/`dpp-vc` — the RNG needs a platform entropy source |
| `wasm32-wasip1` | product group plugins | wasmtime sandbox (WASI P1 syscall interface) |

`getrandom` uses the JS backend for `wasm32-unknown-unknown` (configured in `.cargo/config.toml`).
