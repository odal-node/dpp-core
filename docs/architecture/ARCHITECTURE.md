# dpp-core architecture

## Overview

`dpp-core` is a Rust library for EU Digital Product Passports under the ESPR and the
Battery Regulation. It defines the domain types, cryptographic primitives, schema
validation and port traits: what a passport is, how it is signed, and how compliance is
checked.

No HTTP framework, no database, and no async runtime, except where a port trait needs
one for the application implementing it. The whole workspace compiles with nothing
running.

---

## Module architecture

```text
+--------------------------------------------------+
|                  dpp-domain                       |
|  Domain types, port traits, schema validation     |
|  VersionedSchemaRegistry, ProductGroupCatalog,    |
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
            |     dpp-rules     |   (no_std, no dependencies;
            | cross-field rules |    dpp-domain depends on it)
            +-------------------+
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
+---------------------+  effective periods, receipts.
          |              Depends on dpp-rules for thresholds, never the
          +-> dpp-rules  reverse, since the plugins reach dpp-rules too.

dpp-tests: cross-crate integration tests (not published)
```

The authoritative dependency graph is the one in [CONTRIBUTING.md](../../CONTRIBUTING.md#dependency-rules),
which a test compares with the manifests.

---

## dpp-domain: the domain

The dependency root. Every other crate may depend on it, and it depends on nothing
internal except `dpp-rules`. Six crates do not depend on it; see
[VERSIONING.md](../governance/VERSIONING.md).

### Top-level concerns

This is the largest crate in the workspace and the one most able to absorb a new
capability unnoticed, so its contents are inventoried rather than described. Adding a
module is a deliberate act: `domain_concerns.rs` compares this list with `lib.rs` and
fails the build in either direction.

The **tier** column is the scope rule in [CODE-LAYOUT.md](CODE-LAYOUT.md) §1: imports may
only point up the eight-tier ladder, and two tests hold them to it, one for direction
and one for cycles. The table is ordered by tier, and that order is a measurement of the
import graph, not a judgement about what each module does.

| Concern | Tier | What it holds |
|---|---|---|
| `identifier` | 1 | GS1 and customs-classification vocabulary: `Gtin`, `Gln`, `CommodityCode`, `CnCategory`, and the shared mod-10 check digit. Depends on nothing in this crate |
| `calendar` | 4 | `ObligationCalendar`: which dated obligations take effect between two dates, combining every act's passport start date from `instrument` and every change a `dpp-rules` module schedules, each `ObligationEvent` carrying its `DateBasis`. Holds no clock: the caller passes the window |
| `catalog` | 3 | `ProductGroupCatalog`: identity, scope, schema versions, disclosure classes, plugin binding. It carries **no law** |
| `credential` | 2 | The W3C Verifiable Credential 2.0 envelope that binds a passport to its signed payload |
| `compliance` | 2 | The determination value objects: `ComplianceResult`, its findings, status and error |
| `disclosure` | 2 | The Art. 77(2) lattice: `Audience`, `Disclosure`, and the per-field classification. Tier 2 rather than 3 because `Passport::redact` takes an `Audience`, so the vocabulary cannot sit with the policy that filters by it |
| `eol` | 5 | End-of-life declarations, and the derogation a destruction claim must cite |
| `field_error` | 2 | `FieldError` and `ValidationErrors`: the per-field detail of a validation failure. Split from `error` because the two sit at different levels, and holding them together was a cycle |
| `facility` | 2 | `FacilitySnapshot`: where a product was made, as recorded at issuance |
| `graph` | 5 | The bill-of-materials graph a passport sits in |
| `instrument` | 3 | The legal acts, their `PassportObligation`, and one `InstrumentBinding` per (act, product group) pair. **This is where the law lives** |
| `manufacturer` | 2 | `ManufacturerInfo`: the economic operator that placed the product on the market |
| `material` | 2 | `MaterialEntry`: one declared constituent material |
| `operator` | 2 | `ResponsibleOperator` and the `ResponsibilityBasis` that makes it answerable (Annex III(k)). Shared by `passport` and `transfer`, so owned by neither |
| `personal_data` | 2 | The operator's statement about personal data for a field its schema marks as able to hold it, and the GDPR Art. 6(1) basis a record held outside the passport rests on. Tier 2 because `Passport` carries the statement |
| `passport` | 4 | The aggregate root, its id, reference and audience-filtered view |
| `product` | 5 | `ProductIdentity`: what identifies a product, independent of its passport |
| `product_group` | 4 | The typed per-group payloads and the `ProductGroupData` union |
| `seal` | 2 | eIDAS seal value objects: format, mode, envelope, verification |
| `status` | 2 | `PassportStatus`, the lifecycle state machine |
| `trusted_list` | 2 | EU Trusted List vocabulary (ETSI TS 119 612): service types, statuses, and status at a past moment |
| `transfer` | 5 | Transfer of responsibility between operators |
| `access` | 6 | The per-field disclosure contract: `ProductGroupAccessPolicy`, `filter_by_audience` |
| `lint` | 6 | Non-binding plausibility findings. Never gates publish |
| `schemas` | 3 | `VersionedSchemaRegistry`, the embedded JSON Schemas, and the version lenses |
| `validation` | 6 | Schema conformance and cross-field rules, the pass `Passport::validate` deliberately does not run |
| `ports` | 7 | The boundary between the library and its hosts (see [PORTS.md](PORTS.md)). **Nothing may import it** |
| `passthrough` | 8 | The Apache-2.0 default `ComplianceRegistry` and its per-group strategies: an adapter |
| `error` | 4 | `DppError`, the one error every fallible entry point returns. It sits at the level of the deepest thing it wraps, a lens error from `schemas` |

The concerns are named rather than counted, because a count is what goes stale while
every claim around it stays checkable.

> **A `domain` module existed here until 2026-08-26.** It held 52% of the crate and its
> name said nothing, so nothing in it had been chosen to be there. It was dissolved and
> its children given real homes, which is most of why this table is long: the concerns
> were always this many, they were just not visible.

<!-- DOMAIN-CONCERNS:BEGIN (one module name per line; parsed by domain_concerns.rs) -->
```
access
calendar
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
operator
passport
passthrough
personal_data
ports
product
product_group
schemas
seal
status
transfer
trusted_list
validation
```
<!-- DOMAIN-CONCERNS:END -->

### Domain types

The canonical passport types: `Passport`, `PassportId`, `ManufacturerInfo`,
`MaterialEntry`, `BatteryData`, `TextileData`, `SignedCredential`. All derive `Serialize`
and `Deserialize` and are `wasm32`-safe.

### Port traits

Traits an application implements against its own infrastructure:

| Trait | Async | Purpose |
|---|---|---|
| `PassportRepository` | yes | CRUD for passport records |
| `ComplianceRegistry` | no | Route product group data to the right compliance strategy |
| `ComplianceStrategy` | no | Validate product-group-specific compliance rules |
| `IdentityPort` | yes | Sign and verify passport JWS |
| `PluginHost` | no | Dispatch to Wasm product group plugins |
| `BackupCopyPort` | yes | The ESPR Art. 10(4) third-party back-up copy |
| `ArchivedVersionPort` | yes | The EN 18221 clause 4.2 archive of a passport's historical versions |
| `PersonalDataPort` | yes | Erasable records of personal data held outside a passport |
| `RegistrySyncPort` | yes | EU registry registration and status sync |
| `SealPort` | yes | eIDAS qualified electronic seal (ESPR Art. 13, eIDAS 910/2014) |

The async traits use `async-trait`. The sync traits are plain Rust traits, compatible
with `no_std` and `wasm32`.

`dpp-vc` provides `LocalIdentityService`, an `IdentityPort` implementation backed by
`dpp-crypto`'s local `KeyStore`.

### VersionedSchemaRegistry

Embeds every JSON Schema in `crates/dpp-domain/schemas/{product-group}/v{version}.json`
with `include_str!()` (inside the crate, so they publish with it). It provides:

- `get(product_group, version)`: one schema
- `latest(product_group)`: the newest version for a product group
- `validate(product_group, version, data)`: validate data against a schema
- `list()`: every available (product group, version) pair

Schema validation is gated behind `#[cfg(not(target_arch = "wasm32"))]`, because the
`jsonschema` crate does not support `wasm32`.

---

## dpp-crypto: cryptographic primitives

Signing and key management. No HTTP, no database and **no workspace dependencies**. The
credential and DID layer lives in [`dpp-vc`](#vc-trust), and the per-field
disclosure policy is `dpp_domain::access`. Signing bytes, deciding whose signature means
what, and deciding which fields a role may see are three jobs; this crate does the first.

### KeyStore

Ed25519 key storage encrypted with AES-256-GCM, persisted as a JSON file on the local
filesystem. The path is injected, so tests use temporary directories.

- `open(path, passphrase)`: open or create a key store
- `generate_key(key_id)`: generate a new Ed25519 key pair
- `load_key(key_id)`: load an existing key
- `archive_key(key_id)` and `rotate_key(key_id)`: archive the current key, or archive it
  and generate a replacement
- `load_archived_keys(key_id)`: load every archived key

### Signer

Compact JWS signing (EdDSA with Ed25519):

- `sign(store, key_id, payload, kid)`: a compact JWS whose header names the signing key
  by `kid`
- `verify(store, key_id, jws)`: verify a JWS against a key in the store

### JWS verifier

The single place JWS verification lives:

- `verify_jws(jws, public_key_b64)`: verify a JWS against a base64-encoded public key
- `resolve_verification_key(did_document, jws)`: the Ed25519 public key of the
  verification method the JWS `kid` names, after the Controlled Identifiers v1.0 binding
  checks. No `kid`, no key

---

## dpp-registry: EU registry interface

Data types for the EU Digital Product Passport registry (ESPR Article 13).
`wasm32`-safe: no I/O, no HTTP, no async runtime. It contains wire-format types
(`RegistrationPayload`, `EuRegistryEnvelope`, `EuRegistryResponse`, `StatusResponse`,
`TransferNotification`), the identifier structs (`ProductIdentifier`,
`OperatorIdentifier`, `FacilityIdentifier`, `ProductItemIdentifier`), error types, and
`RegistryEndpoint` with sandbox and production presets.

The port trait (`RegistrySyncPort`) and its stub implementation (`GhostRegistrySync`)
live in `dpp_domain::ports::registry_sync`, not here. A host implements the port with an
HTTP client and these types.

---

<a id="vc-trust"></a>
## dpp-vc: trust

Who may read a passport, and how that is proven: W3C Verifiable Credentials, `did:web`
documents, Bitstring Status List revocation, `LocalIdentityService` (the `IdentityPort`
implementation), and the JSON-LD context these are expressed in. Depends on
`dpp-crypto` for JWS and the keystore.

A credential establishes *which* `Audience` a caller holds; `dpp_domain::access` maps that
audience to fields. Two questions, two crates.

It does not build for `wasm32-unknown-unknown`, because it inherits `dpp-crypto`'s need
for a platform entropy source.

### DID builder

Builds `did:web` documents from the KeyStore state:

- `build_did_document(store, base_url, key_id)`: the full DID document, with one
  `JsonWebKey` verification method per key, identified by the DID and the key's RFC 9278
  thumbprint URI. The current key comes first and alone in `authentication`; it and every
  non-revoked archived key are in `assertionMethod`.

### LocalIdentityService

An implementation of `dpp_domain::ports::IdentityPort` backed by the local `KeyStore`,
combining the signer, DID builder and JWS verifier:

- `sign_passport(passport_id, payload)`: signs the payload with the issuer's Ed25519 key,
  builds the DID document and returns a `SignedCredential`
- `verify_signature(jws, payload)`: resolves the issuer's DID document from the
  `KeyStore` and verifies the JWS

---

## dpp-digital-link: GS1

Stateless, with no I/O or network dependencies. Compiles to `std` and `wasm32`. Two
submodules:

- `digital_link`: GS1 Digital Link URI parsing and building (`DigitalLink::parse` and
  `build`), the GTIN, serial and batch application-identifier table, and QR URL
  construction.
- `linktype`: the GS1 link-type vocabulary and content negotiation (`negotiate`) between
  a client's `Accept` header and a passport's available link descriptors.

---

## dpp-aas: Asset Administration Shell

Maps a `Passport` to an AAS shell and submodels (`build_aas_from_passport`): the
product-group-agnostic core submodels (identification, manufacturer, environmental,
materials, repairability) plus one product group submodel.

**Masked before mapping.** The builder takes an `Audience` and filters the passport
through the same disclosure seam the public view uses, at the entry point, before any
mapper runs. There is no unmasked entry point: a filter applied *after* mapping would
match on `idShort`, so a mapper that misspelled one would emit a restricted field the
filter could not recognise.

**Typed mappers only where an act is in force:** `battery`, `electronics`,
`unsold_goods`, and `textile` as a named exception. Every other product group renders
through the generic projection, because a hand-written mapper asserts an AAS submodel
template that no standards body has ratified. A CI test fails the build if a provisional
product group gains a typed mapper.

**AAS-shaped output with our own semantics.** Every emitted `semanticId` is either
`urn:odal-node:*` or carries a provenance record naming who verified it against the
authority's published source, and a test enforces this. The crate emits no third-party
identifiers today, so no IDTA conformance is claimed.

---

## dpp-plugin-traits: Wasm plugin ABI

Types for the host and guest contract. Uses `std` types (`String`, `Vec`, `HashMap`), so
it is not `no_std`. Product group plugins compiled to `wasm32-wasip1` implement this ABI,
which `dpp-plugin-sdk`'s `export_plugin!` macro generates:

- `alloc(len: u32) -> u32`: allocate `len` bytes, return the pointer
- `dealloc(ptr: u32, len: u32)`: the matching deallocation
- `metadata() -> u64`: `PluginMeta` JSON
- `describe() -> u64`: `PluginCapabilities` JSON (the host runs `check_compatibility`
  before dispatch)
- `validate(ptr, len) -> u64`: the `AbiResult` envelope
- `calculate_metrics(ptr, len) -> u64`: `AbiResult` (`ok: PluginResult`)
- `generate_passport(ptr, len) -> u64`: `AbiResult` (`ok: payload`)

Each `-> u64` packs the output as `(out_ptr << 32) | out_len`. Input and output are UTF-8
JSON in Wasm linear memory.

---

## What is signed and kept

The library validates product data against the product group schema and signs it with
the manufacturer's Ed25519 key. What a host stores and serves is the signed passport: the
product data the operator chose to publish, bound to its signatures. The source files the
data came from (spreadsheets, ERP exports) are not part of it.

Personal data has its own rule. A field a schema marks as able to hold personal data is
refused unless the operator states what is held, and personal data the passport is not
required to carry is kept outside the signed record, where it can be erased when consent
is withdrawn. The `personal_data` module in `dpp-domain` states the rule and the law
behind it, and [PERSONAL-DATA.md](PERSONAL-DATA.md) records the alternatives it was
chosen over.

---

## Wasm targets

| Target | Crates | Purpose |
|---|---|---|
| `wasm32-unknown-unknown` | dpp-domain, dpp-registry, dpp-digital-link, dpp-aas | Browser and edge runtimes hosted by JavaScript. Not `dpp-crypto` or `dpp-vc`: the RNG needs a platform entropy source |
| `wasm32-wasip1` | product group plugins | The wasmtime sandbox (WASI Preview 1) |

`getrandom` uses its JavaScript backend for `wasm32-unknown-unknown`, configured in
`.cargo/config.toml`.
