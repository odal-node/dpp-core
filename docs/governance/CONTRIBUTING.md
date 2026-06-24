# Contributing Guide

This document is for anyone contributing to `dpp-core` — the Odal Node standard library workspace. It covers setup, coding conventions, testing strategy, commit format, and the PR workflow.

---

## 1. Prerequisites

| Tool | Minimum Version | Install |
|---|---|---|
| Rust | see `rust-toolchain.toml` | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| just | latest | `cargo install just` |

Optional but recommended:

| Tool | Purpose |
|---|---|
| `cargo-nextest` | Fast parallel test runner: `cargo install cargo-nextest` |
| `cargo-audit` | Security advisory check: `cargo install cargo-audit` |
| `cargo-watch` | Auto-recompile on file change: `cargo install cargo-watch` |

No Docker, no database, no external services required. This is a pure library.

---

## 2. Local Setup

```bash
git clone https://github.com/odal-node/dpp-core.git
cd dpp-core

cargo build --workspace
cargo nextest run --workspace
just check    # fmt-check + lint + test + audit
```

That's it. Everything compiles and tests with nothing else running.

### `just` Commands

| Command | Description |
|---|---|
| `just check` | Full gate: `fmt-check` + `lint` + `test` + `audit` |
| `just build` | `cargo build --workspace --release` |
| `just test` | `cargo nextest run --workspace` |
| `just lint` | `cargo clippy --workspace --all-targets -- -D warnings` |
| `just fmt` | `cargo fmt --all` |
| `just fmt-check` | CI-safe format check |
| `just audit` | `cargo audit` |
| `just build-plugins` | Compile all Wasm sector plugins (`wasm32-wasip1`) |
| `just clean` | `cargo clean` |

---

## 3. Workspace Structure

```
dpp-core/
  Cargo.toml              # Workspace root — 9 member crates + benches
  LICENSE                  # Apache-2.0
  crates/
    dpp-domain/           # Domain types, port traits, SectorCatalog, VersionedSchemaRegistry
      schemas/            # Versioned JSON schemas, embedded via include_str! (the product):
                          #   aluminium, battery (v1+v2), construction, detergent,
                          #   electronics, furniture, steel, textile (v1+v2),
                          #   textile-unsold, toy, tyre  — 11 sectors
    dpp-rules/            # Pure no_std, zero-dep cross-field regulatory rules
    dpp-crypto/           # Ed25519, AES-GCM, JWS, DID builder, LocalIdentityService
    dpp-digital-link/     # GS1 Digital Link parser, link-type negotiation, AAS mapping
    dpp-calc/             # EU-methodology calculators (CO2e, repairability)
    dpp-plugin-traits/    # Wasm plugin ABI (no_std)
    dpp-plugin-sdk/       # Guest-side SDK: export_plugin! macro + Validator
    dpp-registry/         # EU registry interface types (wasm32-safe)
    dpp-tests/            # Cross-crate integration tests (publish = false)
  benches/                 # Criterion benchmarks (workspace member)
  plugins/                 # 10 sector Wasm plugins (excluded from workspace)
    sector-battery/  sector-textile/  sector-steel/  sector-electronics/
    sector-aluminium/  sector-construction/  sector-detergent/
    sector-furniture/  sector-toy/  sector-tyre/
  docs/                    # Architecture and design documentation
```

### Dependency Rules

The dependency graph is strictly acyclic:

```
dpp-rules         -> (standalone, no internal deps; no_std, zero-dep)
dpp-plugin-traits -> (standalone, no internal deps)
dpp-domain        -> dpp-rules
dpp-crypto        -> dpp-domain
dpp-digital-link  -> dpp-domain
dpp-registry      -> dpp-domain
dpp-calc          -> dpp-domain
dpp-plugin-sdk    -> dpp-plugin-traits + dpp-rules
dpp-tests         -> dpp-domain, dpp-crypto, dpp-digital-link (dev only)
```

**No crate in this workspace may depend on**: axum, tokio, tower, sqlx, redis, reqwest, or any other HTTP/database/infrastructure crate. If a dependency pulls in async runtime or network I/O, it does not belong here.

---

## 4. Coding Conventions

### Pure Domain Code

Every module in this workspace must compile without I/O crates. If you are importing `axum`, `sqlx`, or `async-nats`, that code belongs downstream, not here.

The `KeyStore` in `dpp-crypto` uses `std::fs` for key file persistence — this is accepted as "configuration I/O" (reading a local file the operator placed there), not "business I/O" (calling a database or network service).

### Error Types

Library crates define typed error enums using `thiserror`. No `String` errors in domain code — always use typed variants so callers can pattern-match.

```rust
#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("key not found: {key_id}")]
    KeyNotFound { key_id: String },

    #[error("decryption failed")]
    DecryptionFailed,
}
```

### Logging

All crates use `tracing`. Never log secrets — API keys, private keys, and passphrases must never appear in log output.

### Clippy and Formatting

All code must pass:
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo fmt --all --check`

---

## 5. Testing Strategy

| Test type | Location | Scope |
|---|---|---|
| Unit test | `src/*.rs` (inline `#[cfg(test)]`) | Pure logic, no I/O |
| Integration test | `tests/` directory per crate | Crypto operations with temp files |

All tests run without Docker, without a database, without network access. If a test needs infrastructure, it belongs downstream.

### The `#[cfg(test)]` Fixture Pattern

Domain entities provide `_fixture()` associated functions for test setup, gated behind `#[cfg(test)]` so they never appear in production builds.

---

## 6. Schema Contribution

To add a new schema version for an existing sector:

1. Add `schemas/{sector}/v{new_version}.json`
2. The `VersionedSchemaRegistry` picks it up automatically via `include_str!()`
3. No code changes required — the registry discovers all embedded versions at compile time

To add a new sector:

1. Add `schemas/{sector}/v1.0.0.json`
2. Register the `include_str!()` call in `dpp-core/src/schemas/mod.rs`
3. Add the corresponding validation entry in `dpp-core/src/domain/validation.rs`

---

## 7. Commit Format

All commits follow [Conventional Commits](https://www.conventionalcommits.org/) v1.0.0:

```
<type>(<scope>): <subject>
```

**Types:** `feat`, `fix`, `chore`, `docs`, `refactor`, `test`, `perf`, `security`

**Scopes:** `domain`, `rules`, `crypto`, `digital-link`, `calc`, `plugin-traits`, `plugin-sdk`, `registry`, `schemas`, `ci`, `docs`

**Examples:**
```
feat(crypto): add JWS verification for archived key rotation
fix(core): handle empty sector string in VersionedSchemaRegistry
docs(schemas): add textile v1.1.0 schema for ESPR amendment
chore(deps): upgrade ed25519-dalek to 2.2.1
```

Breaking changes use `!` after the scope: `feat(domain)!: remove deprecated field from Passport`.

### Developer Certificate of Origin (DCO)

All commits must be signed off under the
[Developer Certificate of Origin v1.1](https://developercertificate.org/).
By adding a `Signed-off-by:` line, you certify that you wrote or have the
right to submit the contribution under the project's Apache-2.0 license.

Add it automatically with the `-s` flag:

```bash
git commit -s -m "feat(domain): add new port trait"
```

This produces:

```
feat(domain): add new port trait

Signed-off-by: Your Name <your.email@example.com>
```

CI will reject commits missing the sign-off line.

---

## 9. Pull Request Workflow

- Every change goes through a PR. No direct pushes to `main`.
- All CI checks must pass: build, test, clippy, audit, fmt.
- PRs that modify port traits must explain the downstream impact.
- PRs that add a new dependency must state why in the PR description.
- PR titles follow the same Conventional Commits format.

### Branch Names

```
feat/schema-registry-versioning
fix/crypto-key-rotation-archive
docs/wasm-target-guide
```

---

## 10. Security Practices

- **No hardcoded secrets** anywhere, including tests. Use temp files with random names.
- **No `unsafe` code** without an explicit ADR.
- **No `unwrap()` or `expect()` in library code paths.** Only in tests.
- **`cargo audit`** must pass in CI.

Report security vulnerabilities to **security@odal-node.io** — do not open public issues.
