# Odal Node Core — Blueprint

## Vision

dpp-core is the open-source standard library for EU ESPR Digital Product Passport compliance. It is the standard, not the product. Anyone building DPP tooling can use it as the foundation.

The goal is a single, well-audited implementation of the regulatory standard in Rust — domain types, cryptographic signing, schema validation, GS1 resolution, W3C Verifiable Credentials, and the port traits that define the boundary between the standard and any infrastructure.

---

## Guiding Principles

### 1. Proof-Bound Architecture

Raw production data never enters Odal's systems. The library validates product data against the sector schema, signs it with the manufacturer's Ed25519 key, and produces a cryptographically verifiable proof. The raw data is the manufacturer's responsibility. The signed proof is what gets persisted and served.

This is an architectural constraint, not a feature.

### 2. Standards-First

Every technical decision starts from an open standard. No proprietary formats, no invented protocols. The standards the library implements:

- GS1 Digital Link v1.2 — product identification and QR resolution
- W3C Verifiable Credentials v2.0 — access control and manufacturer identity
- IDTA Asset Administration Shell v3.0 — Industry 4.0 / Catena-X interoperability
- CEN/CENELEC JTC 24 — EU DPP data model (tracked as it evolves)
- did:web — decentralised manufacturer identity over DNS + HTTPS

### 3. The Compilation Test

`cargo build --workspace` must succeed with zero infrastructure running — no database, no Redis, no environment variables. If code changes because infrastructure changed, it does not belong here.

### 4. The Golden Rule

> If code changes because an EU regulation changed → it belongs in **dpp-core**.
> If code changes because the business model changed → it belongs in **dpp-engine**.

This separation keeps the regulatory implementation open, auditable, and free to use, regardless of what happens on the commercial side.

### 5. Open-Core Boundary

The boundary is a Rust trait (`ComplianceRegistry`). It is a **technical extension seam**, not a commercial paywall — compliance calculation is open (Apache-2.0 Wasm plugins and the platform's calculators). Every piece of code that implements an EU regulation or open standard is Apache-2.0. Revenue comes from operation, regulatory currency, and trust services, not from gating capability.

### 6. Port Traits as the Extension Contract

The port traits in `dpp-domain::ports` define the complete interface between the standard and any implementation. Anyone who implements these traits against their own infrastructure has a complete, standard-compliant DPP system. The core has no opinion on database, cache, or HTTP framework.

---

## Non-Goals For Now

The following are explicitly out of scope for dpp-core:

- HTTP framework, database, or async runtime (belongs in the platform)
- Multi-tenancy, authentication, or API key management (platform concerns)
- Audit log storage (platform concern — the core enforces lifecycle rules, not storage)
- Blockchain anchoring
- Native mobile app
- IoT sensor integration
- AI/ML features
- Direct ERP connectors (CSV import is the integration strategy)
- Consumer tracking analytics
- Any code that requires infrastructure to compile
