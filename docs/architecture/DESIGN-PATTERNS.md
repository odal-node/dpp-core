# Design patterns

The architectural patterns `dpp-core` uses, and the reason for each.

---

## 1. Hexagonal architecture (ports and adapters)

Domain logic is isolated from everything external. Every external dependency sits
behind a trait.

```text
         +---------------------------------------------+
         |             Driving adapters                |
         |  (HTTP handlers, CLI, test doubles)         |
         +---------------------+-----------------------+
                               |  <- driving ports (Rust traits)
                   +-----------v-----------+
                   |    Domain             |
                   |  (dpp-core)           |
                   |  - passport validation|
                   |  - lifecycle rules    |
                   |  - signing logic      |
                   +-----------+-----------+
                               |  <- driven ports (Rust traits)
         +---------------------v-----------------------+
         |             Driven adapters                 |
         |  (database, cache, event bus, file storage) |
         +---------------------------------------------+
```

**The domain** is plain structs, enums and functions in `dpp-domain`, with no I/O.

**Driven ports** are the traits in `dpp_domain::ports` that the domain uses to reach
infrastructure. An excerpt of `PassportRepository`:

```rust
#[async_trait]
pub trait PassportRepository: Send + Sync {
    async fn create(&self, passport: Passport) -> Result<Passport, DppError>;
    async fn find_by_id(&self, id: PassportId) -> Result<Option<Passport>, DppError>;
    async fn update(&self, passport: Passport) -> Result<Passport, DppError>;
    // … lookups by identifier, carrier, identity and successor
}
```

Adapter implementations live in your application. The domain is tested with in-memory
implementations, without containers or network calls.

---

## 2. The compliance extension point

`ComplianceRegistry` in `dpp_domain::ports` is the single point where
product-group-specific compliance logic plugs in. It is a technical boundary: the
compliance logic itself is open (Apache-2.0 Wasm plugins and the calculators in
`dpp-calc`).

```rust
pub trait ComplianceRegistry: Send + Sync {
    fn compute(
        &self,
        product_group_key: &str,
        data: &ProductGroupData,
        law_in_force_on: Option<NaiveDate>,
    ) -> Result<ComplianceResult, ComplianceError>;
}

pub trait ComplianceStrategy: Send + Sync {
    fn product_group_key(&self) -> &str;
    fn compute(
        &self,
        data: &ProductGroupData,
        law_in_force_on: Option<NaiveDate>,
    ) -> Result<ComplianceResult, ComplianceError>;
}
```

The default is `PassthroughRegistry`, which accepts manufacturer-supplied values verbatim
and reports `PassthroughNoValidation`. Wasm product group plugins, or calculators wired in
by your application, provide real validation. Any implementation can be injected at
startup as an `Arc<dyn ComplianceRegistry>`, with no change to domain code.

`law_in_force_on` is the date whose law governs the product, normally its placing-on-market
date, never the wall clock. `None` means the date is unknown, and a strategy that needs it
says so rather than guessing.

---

## 3. Stub implementations behind real ports

Some ports front external services that an operator may not have configured yet, or that
cannot have legal effect yet: the EU registry, a qualified seal provider, an independent
back-up host. For each, the port is fully specified now, and the library ships a stub:

| Port | Stub | Behaviour |
|---|---|---|
| `RegistrySyncPort` | `GhostRegistrySync` | Accepts every submission under a `GHOST-CORR-` correlation id, sends nothing, and knows of no submission afterwards |
| `SealPort` | `GhostSeal` | Returns an unsigned envelope marked `placeholder: true`, whose verification is indeterminate |
| `BackupCopyPort` | `GhostBackup` | Returns a receipt with a `GHOST-` id and an empty content hash, and stores nothing |

Each stub marks its output as a placeholder (a `placeholder: true` flag or a synthetic
`GHOST-` identifier), so a host can detect it at runtime and refuse to run on it in
production. The stubs are always compiled in rather than feature-gated: one public path
per port is simpler to use, and the runtime markers do the job either way.

`RegistrySyncPort` itself reflects the registry as published: `register` returns an
acknowledgement, not a registration, and `check_submission`, `find_submission`,
`confirm_submission` and `check_status` follow the asynchronous validation Implementing
Regulation (EU) 2026/1778 describes.

---

## 4. Schemas as code

Versioned JSON Schemas are embedded at compile time with `include_str!()`, from
`crates/dpp-domain/schemas/{product-group}/v{version}.json`, through the table in
`crates/dpp-domain/src/schemas/embedded.rs`. `VersionedSchemaRegistry` provides:

- `get(product_group, version)`: one schema
- `latest(product_group)`: the newest version for a product group
- `validate(product_group, version, data)`: validate data against a schema

A new version is a new file and one entry in the embedded table. Schema changes are
version-controlled, reviewed and tested like any other code, and
[SCHEMA-CHANGES.md](SCHEMA-CHANGES.md) is regenerated from the schemas themselves.

---

## 5. Port traits as the contract

The port traits in `dpp_domain::ports` are the full contract between the library and any
application. [PORTS.md](PORTS.md) lists each one, and a test keeps that list equal to the
code.

Implementing these traits against your own infrastructure gives a complete passport
system. The library has no opinion on the database, cache or HTTP framework.
Authentication, API keys and isolation between operators belong to your application and
how it is deployed; none of them belong in the library.
