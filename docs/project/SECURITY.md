# Security Policy

## Reporting a Vulnerability

**Do NOT open a public GitHub issue for security vulnerabilities.**

Report vulnerabilities privately to **security@odal-node.io** with:

1. A description of the vulnerability and its potential impact
2. Steps to reproduce or a minimal proof of concept
3. The affected crate(s) and version(s)
4. Any suggested fix or mitigation, if you have one

## What to Expect

| Step | Timeframe |
|------|-----------|
| Acknowledgement of your report | Within 48 hours |
| Initial assessment and severity classification | Within 5 business days |
| Fix or mitigation for critical/high severity | Within 14 days |
| Fix or mitigation for medium/low severity | Within 30 days |
| Public disclosure (coordinated with reporter) | After fix is released |

We follow coordinated vulnerability disclosure (CVD) as recommended by the [OpenSSF Vulnerability Disclosure Working Group](https://github.com/ossf/wg-vulnerability-disclosures). We will work with you on timing and credit.

## Scope

This policy covers all crates in the dpp-core workspace:

| Crate | Security-Relevant Surface |
|-------|---------------------------|
| **dpp-crypto** | Ed25519 key management, AES-256-GCM encryption, JWS signing/verification, Verifiable Credential issuance |
| **dpp-domain** | Access tier policy enforcement, schema validation, transfer chain integrity |
| **dpp-digital-link** | GS1 Digital Link URI parsing (input validation) |
| **dpp-plugin-traits** | Wasm plugin ABI boundary |
| **dpp-registry** | EU Registry interface types |

Issues in the following areas are particularly important:

- Cryptographic key leakage or weak randomness
- JWS signature bypass or forgery
- Access tier escalation (e.g., public credentials accessing confidential data)
- Schema validation bypass allowing non-compliant passports
- Transfer chain integrity violations (skipping states, forging history)

## Out of Scope

- Issues in the dpp-engine repository (report to the same email, but this policy covers dpp-core only)
- Vulnerabilities in upstream dependencies (report to the dependency maintainer; we monitor via `cargo audit` in CI)
- Feature requests or non-security bugs (use GitHub Issues)

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.1.x (current) | Yes |

Only the latest release receives security patches. When a new minor or major version is released, the previous version receives security patches for 90 days, then support ends.

## Recognition

We credit security researchers in the CHANGELOG and release notes (unless you prefer to remain anonymous). We do not currently operate a bug bounty programme.

## Security Tooling in CI

The following automated checks run on every push and pull request:

- `cargo audit` — checks all dependencies against the [RustSec Advisory Database](https://rustsec.org/)
- `cargo clippy -- -D warnings` — catches common correctness issues
- `cargo nextest run` — runs the full test suite including cryptographic verification tests

## Cryptographic Design Decisions

dpp-core uses the following cryptographic primitives:

| Purpose | Algorithm | Crate | Rationale |
|---------|-----------|-------|-----------|
| Passport signing | Ed25519 | `ed25519-dalek` | ESPR-aligned, deterministic, fast, 128-bit security level |
| Field encryption | AES-256-GCM | `aes-gcm` | Authenticated encryption for confidential passport fields |
| Hashing | SHA-256 | `sha2` | Content-addressable passport identifiers |
| Key derivation | N/A (direct key generation) | `rand` (OS entropy) | Keys are generated, not derived from passwords |

No custom cryptography is implemented. All primitives are from audited, widely-used Rust crates.
