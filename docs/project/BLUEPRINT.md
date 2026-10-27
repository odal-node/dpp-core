# dpp-core blueprint

## Purpose

`dpp-core` is an open-source Rust library for EU Digital Product Passports under the ESPR
and the Battery Regulation. Anyone building passport tooling can use it as a foundation.

The aim is one well-reviewed implementation of the regulatory model in Rust: domain types,
cryptographic signing, schema validation, product identifiers and GS1 Digital Link, W3C
Verifiable Credentials, and the port traits that separate the model from any
infrastructure.

---

## Design rules

### 1. Sign at the source

The library validates product data against the product group schema and signs it with the
manufacturer's Ed25519 key. What an application stores and serves is the signed passport:
the data the operator chose to publish, bound to its signatures. Source files (spreadsheets,
ERP exports) are not part of it, and a verifier needs nothing from the issuer's systems
except its public `did:web` document.

### 2. Open standards first

Every technical decision starts from an open standard. No proprietary formats or invented
protocols. The library implements:

- EN 18219 product identifiers: a GS1 Digital Link, an IEC 61406 identification link or
  a decentralised identifier (DID), with GS1 Digital Link parsing and QR resolution
- W3C Verifiable Credentials: access control and manufacturer identity
- IDTA Asset Administration Shell: AAS-shaped output for Industry 4.0 tooling
- CEN/CENELEC JTC 24: the EU passport data model, tracked as it evolves
- `did:web`: manufacturer identity over DNS and HTTPS

This list does not record which revision of each is cited, its status, or the evidence
behind the implementation. That is recorded once, in
[`docs/architecture/STANDARDS.md`](../architecture/STANDARDS.md), with whether conformance
is claimed.

### 3. Builds with nothing running

`cargo build --workspace` must succeed with no infrastructure: no database, no Redis, no
environment variables.

### 4. Regulation here, operation elsewhere

> If code changes because an EU regulation changed, it belongs in `dpp-core`.
> If code changes because of how a system is deployed or operated, it belongs in the
> application that deploys it.

This keeps the regulatory implementation open, reviewable and free to use.

### 5. One extension point for compliance

The boundary is a Rust trait, `ComplianceRegistry`. It is a technical extension point:
compliance calculation is open (Apache-2.0 Wasm plugins and the `dpp-calc` calculators).
Every piece of code that implements an EU regulation or open standard is Apache-2.0.

### 6. Port traits as the contract

The port traits in `dpp_domain::ports` define the whole interface between the library and
any implementation. Implementing them against your own infrastructure gives a complete
passport system. The library has no opinion on the database, cache or HTTP framework.

---

## Out of scope

- HTTP frameworks, databases and async runtimes (they belong to the application)
- Multi-tenancy, authentication and API key management
- Audit log storage (the library enforces lifecycle rules, not storage)
- Blockchain anchoring
- Native mobile apps
- IoT sensor integration
- AI and machine-learning features
- Direct ERP connectors (structured import is the integration path)
- Consumer tracking analytics
- Any code that needs infrastructure to compile
