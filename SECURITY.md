# Security policy

## Report a vulnerability

**Do not open a public issue.** Report privately through **Report a vulnerability** on
this repository's **Security** tab, or by email to **security@odal-node.io**, with:

1. What the vulnerability is and what it allows.
2. Steps to reproduce, or a minimal proof of concept.
3. The affected crates and versions.
4. A suggested fix or mitigation, if you have one.

## Response times

| Step | Target |
|---|---|
| Acknowledgement | 48 hours |
| Initial assessment and severity | 5 business days |
| Fix or mitigation, critical or high severity | 14 days |
| Fix or mitigation, medium or low severity | 30 days |
| Public disclosure | After the fix is released, coordinated with you |

We follow coordinated disclosure as described by the
[OpenSSF Vulnerability Disclosure Working Group](https://github.com/ossf/wg-vulnerability-disclosures)
and agree timing and credit with you.

## Scope

This policy covers every crate published from this repository.

| Crate | Security-relevant surface |
|---|---|
| `dpp-crypto` | Ed25519 keys, JWS signing and verification, the encrypted keystore, JAdES construction, SD-JWT selective disclosure |
| `dpp-vc` | Verifiable Credential issuance and verification, `did:web` documents, status lists |
| `dpp-domain` | Disclosure classes and redaction, schema validation, transfer-chain integrity |
| `dpp-digital-link` | GS1 Digital Link parsing (untrusted input) |
| `dpp-aas` | Audience filtering of the AAS projection |
| `dpp-plugin-traits`, `dpp-plugin-sdk` | The Wasm plugin ABI boundary |
| `dpp-registry` | EU registry request and response types |
| `dpp-rules`, `dpp-calc`, `dpp-vocab` | Rule and calculation correctness, vocabulary records |

Findings that matter most:

- Key leakage or weak randomness.
- JWS signature bypass or forgery.
- **Audience escalation:** a public caller reading `Restricted`, `Conformity` or
  `Individual` data, or an authority credential reaching Annex XIII point 4
  individual-battery data it is not entitled to.
- Schema validation bypass that lets a non-compliant passport through.
- Transfer-chain integrity violations: skipped states or forged history.

## Out of scope

- **Software that uses these crates.** This policy covers only the crates published from
  this repository. Report issues in an application to that application's security contact.
- **Vulnerabilities in upstream dependencies.** Report them to the dependency's
  maintainers and to [RustSec](https://rustsec.org/). `cargo audit` runs in CI and on a
  daily schedule, and a patched release follows once an upstream fix exists.
- **Feature requests and ordinary bugs.** Use GitHub Issues.

## Supported versions

Security fixes go into the **latest release**. After a new minor or major version, the
previous minor version receives security fixes for **90 days**.

Every crate in the workspace is published to crates.io in lockstep under one version;
[`dpp-domain`](https://crates.io/crates/dpp-domain) shows the current one. There is no
`dpp-core` crate (that is the workspace name). Each release is a `vX.Y.Z` git tag with a
matching entry on the [Releases page](https://github.com/odal-node/dpp-core/releases).

## Credit

Reporters are credited in the CHANGELOG and release notes unless they prefer to stay
anonymous. There is no bug bounty.

## Security checks in CI

On every push and pull request:

- `cargo audit` against the [RustSec Advisory Database](https://rustsec.org/), and again
  daily on a schedule.
- `cargo clippy` with warnings as errors.
- The full test suite, including RFC test vectors for Ed25519 (RFC 8032), JOSE (RFC 8037),
  JCS (RFC 8785) and Argon2 (RFC 9106).

## Cryptographic primitives

| Purpose | Algorithm | Crate |
|---|---|---|
| Passport and credential signatures | Ed25519 (`alg: EdDSA`) | `ed25519-dalek` |
| Keystore encryption of private keys at rest | AES-256-GCM | `aes-gcm` |
| Keystore key derivation from the passphrase | Argon2id | `argon2` |
| Keystore record integrity | HMAC-SHA256 | `hmac`, `sha2` |
| Hashing (JWK thumbprints, content digests, hash chains) | SHA-256 | `sha2` |
| Canonical JSON before signing | JCS (RFC 8785) | `serde_jcs` |
| Signing key generation | OS entropy | `rand` |

No primitive is implemented here. All of them come from widely used Rust crates. The
keystore design has not been externally audited.
