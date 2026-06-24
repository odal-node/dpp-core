# Domain Architecture Overview

This document describes the Odal DPP domain: what a Digital Product Passport is, how it flows through creation, signing, and resolution, and what trust guarantees the system provides.

This is the domain model — infrastructure-agnostic. Any implementation that correctly implements the port traits in `dpp-domain::ports` produces a compliant DPP system.

---

## 1. Architectural Philosophy

Odal is built on **Hexagonal Architecture** (Ports & Adapters). The domain logic — schema validation, lifecycle transitions, signing orchestration — is isolated from all external concerns.

The domain is a set of pure Rust functions and trait definitions. Every external dependency (database, cache, event bus, file storage) is behind a trait. This means:

- The domain can be tested with in-memory stubs — no containers, no network calls
- Any infrastructure component can be replaced without touching regulatory business logic
- The same domain code compiles to native, `wasm32-unknown-unknown`, and `wasm32-wasip1`

---

## 2. DPP Lifecycle

Every passport follows this state machine:

```
Draft  -->  Active (Published)  -->  Suspended  -->  Archived
  |                                      |
  +--------------------------------------+
                  (can also archive directly)
```

| Transition | Precondition | Side Effect |
|---|---|---|
| Draft -> Active | All mandatory fields present and valid | Signed with issuer's Ed25519 key; JWS produced |
| Active -> Suspended | Reason required (recall, investigation, etc.) | Signature retained; resolver returns 410 |
| Any -> Archived | Irreversible | Retained for regulatory lifecycle (10-20 years) |

Every transition is recorded by the platform layer (audit logging is a platform concern, not a domain concern).

---

## 3. Data Flow (Conceptual)

```
WRITE PATH
----------
1. Manufacturer submits product data (JSON or CSV)
2. Data validated against sector JSON Schema (VersionedSchemaRegistry)
3. Compliance checks run via ComplianceRegistry (Wasm plugin or passthrough)
4. Validated data persisted via PassportRepository
5. On publish: signed with Ed25519 via IdentityPort -> JWS compact serialisation
6. EU bridge notified (currently no-op)

READ PATH
---------
1. Consumer scans QR code on product
2. Resolver fetches passport via PassportRepository
3. JWS signature verified against manufacturer's did:web DID document
4. Passport served with verification status
```

---

## 4. Trust Anchor

**Problem:** When a consumer scans a QR code, how do they know the DPP data is authentic?

**Solution: JWS-Signed QR URL with did:web Verification**

Every published passport carries a JWS compact serialisation signed with the manufacturer's Ed25519 private key. The corresponding public key is published in the manufacturer's `did:web` DID document at `/.well-known/did.json`.

Verification:
1. Fetch the manufacturer's DID document
2. Extract the Ed25519 public key for the verification method referenced in the JWS header
3. Verify the JWS signature over the payload
4. If valid: the data is byte-for-byte what the manufacturer signed

**Trust root**: DNS + HTTPS (`did:web`) — the same infrastructure that secures the manufacturer's own website.

**What this does NOT guarantee**: That the manufacturer's data is accurate. They may have entered incorrect values. Odal provides the pipe, not the truth.

---

## 5. Key Rotation

Key rotation is a first-class operation:

1. The current Ed25519 key is archived with a timestamp
2. A new key pair is generated and becomes `#key-1` (primary, authentication)
3. Archived keys are retained as `#key-2`, `#key-3`, etc. under `assertionMethod` in the DID document
4. All past JWS signatures remain verifiable against their corresponding archived key

This means a manufacturer can rotate keys without invalidating any previously published passports.

---

## 6. Compliance Model

The `ComplianceRegistry` trait is the determination seam. The library ships the sector-agnostic `PassthroughRegistry`:

```
ComplianceRegistry
  |
  +-- PassthroughRegistry (dpp-core, Apache) -> PassthroughNoValidation for every sector
  +-- plugin-backed registry (platform)      -> Wasm sector plugins (real metrics)
  +-- PremiumComplianceRegistry (proprietary)-> paid calculators
```

`PassthroughRegistry` computes nothing — it accepts manufacturer-supplied values verbatim and returns `PassthroughNoValidation` for every sector. Real compliance validation (CBAM thresholds, ESPR Article 22, Battery Regulation checks) comes from the Wasm sector plugins via a plugin-backed registry in the platform. A computed determination is passed through `gate_determination(catalog.is_in_force(sector), …)` so a provisional sector can never surface a binding `Compliant`/`NonCompliant`. See `PLUGIN-HOST.md` and `SECTOR-MODEL-CONSOLIDATION.md` §3.1.

This trait is the extension seam. Any implementation — open Wasm plugins, the platform's calculators, or third-party modules — can replace or extend it without touching any other code. It is a technical boundary, not a commercial one.

---

## 7. Schema Versioning

Schemas follow semver. The `VersionedSchemaRegistry` supports n+1 versioning:

```
schemas/
  battery/v1.0.0.json
  textile/v1.0.0.json
  textile/v1.1.0.json      # minor version adds optional fields
  textile-unsold/v1.0.0.json
  steel/v1.0.0.json
```

Adding a new schema version is a single file addition. The registry discovers all embedded versions at compile time. Validation callers specify which version to validate against, or request the latest.

---

## 8. Issuer Model

The core library is single-issuer and stateless — it has no concept of tenants, authentication, or API keys. Each issuer is identified by a `key_id` that maps to an Ed25519 key pair in the `KeyStore`, and by a `did:web` DID derived from the issuer's domain.

Multi-tenancy, authentication, and access control are platform concerns — they live in the `dpp-engine` repository, not in the core.

---

## 9. Port Traits — The Extension Contract

These traits define the boundary between the standard and any implementation:

```rust
// Persistence
trait PassportRepository    // CRUD for DPP records

// Compliance
trait ComplianceRegistry    // Route to sector strategy
trait ComplianceStrategy    // Sector-specific validation

// Identity
trait IdentityPort          // Sign and verify JWS

// Plugins
trait PluginHost            // Dispatch to Wasm plugins

// Archival & registry & sealing
trait ArchivePort           // Immutable DPP archival with retention guarantees
trait RegistrySyncPort      // EU Central Registry registration / status sync
trait SealPort              // eIDAS qualified electronic seal (ESPR Art. 13)
```

`PassportRepository`, `IdentityPort`, `ArchivePort`, `RegistrySyncPort`, and `SealPort` are `async`. The compliance and plugin traits are sync — they must work in `no_std` and `wasm32` contexts.

`dpp-crypto` provides `LocalIdentityService`, a concrete `IdentityPort` implementation backed by the local `KeyStore`. Anyone who implements the remaining traits against their own infrastructure has a complete, standard-compliant DPP system.
