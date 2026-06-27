# Design Patterns

This document describes the architectural patterns used in dpp-core and the reasoning behind each choice.

---

## 1. Hexagonal Architecture (Ports & Adapters)

The domain logic is isolated from all external concerns. Every external dependency is behind a trait.

```
         +---------------------------------------------+
         |             Driving Adapters                 |
         |  (HTTP handlers, CLI, test stubs)            |
         +---------------------+------------------------+
                               |  <- Driving Ports (Rust traits)
                   +-----------v-----------+
                   |    Domain / Core      |
                   |  (dpp-core)          |
                   |  - DPP validation     |
                   |  - Lifecycle rules    |
                   |  - Signing logic      |
                   |  - Audit records      |
                   +-----------+-----------+
                               |  <- Driven Ports (Rust traits)
         +---------------------v------------------------+
         |             Driven Adapters                  |
         |  (Database, cache, event bus, file storage)  |
         +---------------------------------------------+
```

**Domain** = pure structs, enums, and functions in `dpp-domain`. Zero async, zero I/O.

**Driven Ports** = Rust `trait`s in `dpp-domain::ports` that the domain uses to reach infrastructure:
```rust
pub trait PassportRepository: Send + Sync {
    async fn create(&self, passport: Passport) -> Result<Passport, DppError>;
    async fn find_by_id(&self, id: PassportId) -> Result<Option<Passport>, DppError>;
    async fn update(&self, passport: Passport) -> Result<Passport, DppError>;
}
```

All adapter implementations live downstream. The domain is testable with in-memory stubs — no containers, no network calls.

---

## 2. Open-Core Boundary

The `ComplianceRegistry` trait in `dpp-domain::ports` is the **extension seam** — the single point where sector-specific compliance logic plugs in. It is a *technical* boundary: compliance calculation is open (Apache-2.0 Wasm plugins and the engine's calculators).

```rust
pub trait ComplianceRegistry: Send + Sync {
    fn get_strategy(&self, sector: &str) -> Option<&dyn ComplianceStrategy>;
}

pub trait ComplianceStrategy: Send + Sync {
    fn validate(&self, data: &serde_json::Value) -> Result<(), ComplianceError>;
}
```

dpp-domain ships with `PassthroughNoValidation` — it accepts manufacturer-supplied compliance values verbatim. Wasm sector plugins or the platform's calculators provide real compliance validation.

Any compliance engine implementing this trait can be injected at startup via dynamic dispatch without any changes to the domain code.

---

## 3. Ghost Bridge

The `dpp-registry` crate represents a dependency on a future external system (the EU EUDPP Central Registry) that does not yet exist. The pattern: **define the interface now; implement it when the dependency arrives.**

```rust
pub trait EudppBridge: Send + Sync {
    async fn register_passport(&self, dpp_id: &PassportId, vc: &VerifiableCredential)
        -> Result<BridgeResult, BridgeError>;
    async fn update_passport(&self, dpp_id: &PassportId, vc: &VerifiableCredential)
        -> Result<BridgeResult, BridgeError>;
    async fn revoke_passport(&self, dpp_id: &PassportId, reason: &str)
        -> Result<BridgeResult, BridgeError>;
}

pub struct GhostRegistrySync;  // Returns Ok(Pending) for all calls
```

When the EU publishes the EUDPP API specification, `dpp-registry` gets a real implementation. Zero other crates change — the bridge call is already wired downstream.

---

## 4. Schema-as-Code

Versioned JSON schemas are embedded at compile time via `include_str!()`. The `VersionedSchemaRegistry` discovers all schemas in `schemas/{sector}/v{version}.json` and provides:

- `get(sector, version)` — retrieve a specific schema
- `latest(sector)` — retrieve the newest version for a sector
- `validate(sector, version, data)` — validate passport data

Adding a new schema version is a single file addition. No code changes required — the registry discovers all embedded versions automatically.

This means schema changes are tracked in version control, reviewed in PRs, and tested in CI like any other code change.

---

## 5. Port Traits as Extension Contract

The port traits in `dpp-domain::ports` form the complete contract between the standard and any implementation:

| Trait | Async | Purpose |
|---|---|---|
| `PassportRepository` | yes | CRUD for DPP records |
| `ComplianceRegistry` | no | Route to sector strategy |
| `ComplianceStrategy` | no | Sector-specific validation |
| `IdentityPort` | yes | Sign and verify JWS |
| `PluginHost` | no | Dispatch to Wasm plugins |

`PassportRepository` and `IdentityPort` use `async-trait`. Non-async traits are plain Rust traits — compatible with `no_std` and `wasm32`.

Anyone who implements these traits against their own infrastructure has a complete, standard-compliant DPP system. The core library has no opinion on what database, cache, or HTTP framework is used. Authentication and API key management are platform concerns, and multi-operator isolation is an infrastructure concern (one node per operator) — none belong in the core.
