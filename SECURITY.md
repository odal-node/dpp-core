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
| **dpp-domain** | Disclosure classification and redaction, schema validation, transfer chain integrity |
| **dpp-digital-link** | GS1 Digital Link URI parsing (input validation) |
| **dpp-plugin-traits** | Wasm plugin ABI boundary |
| **dpp-registry** | EU Registry interface types |

Issues in the following areas are particularly important:

- Cryptographic key leakage or weak randomness
- JWS signature bypass or forgery
- Audience escalation (e.g. a public caller reading `Restricted`, `Conformity`
  or `Individual` data, or an authority credential reaching Annex XIII point 4
  individual-item data it is not entitled to)
- Schema validation bypass allowing non-compliant passports
- Transfer chain integrity violations (skipping states, forging history)

## Out of Scope

- Issues in software that consumes these crates — this policy covers the crates published from this repository only, so report those to that project's own security contact
- **Vulnerabilities in upstream dependencies** — report these to the dependency maintainer and to [RustSec](https://rustsec.org/). We monitor advisories via `cargo audit`, both in CI and on a daily schedule, and will publish a patched release once a fix is available upstream. We cannot fix them ourselves, which is why they are out of scope for this policy rather than out of mind.
- Feature requests or non-security bugs (use GitHub Issues)

## Supported Versions

Security patches are issued for the **latest tagged release**. When a new minor or major version is released, the previous minor continues to receive security patches for **90 days**, after which support for it ends.

The current version is on the [tags page](https://github.com/odal-node/dpp-core/tags), and on crates.io, where every crate in the workspace is published in lockstep under one version — [`dpp-domain`](https://crates.io/crates/dpp-domain) names it. (There is no `dpp-core` crate: that is the workspace, not a published artifact. GitHub Releases is not the pointer either — this repository tags its releases and does not publish release objects, so `/releases/latest` resolves to an empty page.) This section names no version number on purpose: the previous wording hardcoded one, and it went on asserting `0.1.x` across the nineteen minor releases from 0.2 to 0.20, because nothing breaks when a policy document goes stale.

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
