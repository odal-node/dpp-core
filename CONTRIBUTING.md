# Contributing

How to build, test and change `dpp-core`. For the organization-wide rules (issues
first, primary sources, conduct), see the
[organization guide](https://github.com/odal-node/.github/blob/main/CONTRIBUTING.md).
Documentation follows the
[documentation style guide](https://github.com/odal-node/.github/blob/main/DOCS-STYLE.md).

## 1. Prerequisites

| Tool | Version | Install |
|---|---|---|
| Rust | pinned in `rust-toolchain.toml` | [rustup.rs](https://rustup.rs) |
| just | latest | `cargo install just` |
| cargo-nextest | latest | `cargo install cargo-nextest` |
| cargo-audit | latest | `cargo install cargo-audit` |

The Wasm targets (`wasm32-wasip1` for plugins, `wasm32-unknown-unknown` for the
wasm-safe crates) are only needed to build plugins or run `just wasm-build`.

No Docker, database or external service is needed.

## 2. Build and test

```bash
git clone https://github.com/odal-node/dpp-core.git
cd dpp-core
cargo build --workspace
cargo nextest run --workspace
just check
```

| Command | What it runs |
|---|---|
| `just check` | The local gate: format, clippy, plugin format and clippy, tests, doctests (each crate's README is compiled), plugin tests, plugin gate self-test, docs, lockfile check, `cargo audit` |
| `just ci` | Everything CI runs, plus docs. Run it before pushing |
| `just test` | `cargo nextest run --workspace` (no doctests) |
| `just test-doc` | Doctests, including the crate READMEs |
| `just lint` | `cargo clippy --workspace --all-targets --all-features -- -D warnings` |
| `just fmt` | `cargo fmt --all` |
| `just build-plugins` | Build every Wasm product group plugin for `wasm32-wasip1` |
| `just wasm-build` | Cross-compile the crates that must stay wasm-safe |
| `just schema-changes` | Regenerate `docs/architecture/SCHEMA-CHANGES.md` from the schemas |

`just --list` shows every recipe. `just check` does not cross-compile, so the Wasm
jobs can still fail in CI after it passes locally.

On a 16 GB machine, `just check` can run out of memory at full parallelism. Set
`CARGO_BUILD_JOBS=3` if it does.

## 3. Workspace layout

```text
dpp-core/
  Cargo.toml            workspace root: 12 member crates and benches
  crates/
    dpp-domain/         domain types, port traits, catalogs, VersionedSchemaRegistry
      product-groups/   one manifest per product group
      instruments/      one manifest per EU act
      schemas/          versioned JSON Schemas, embedded with include_str!
    dpp-rules/          no_std, zero-dependency cross-field regulatory rules
    dpp-crypto/         Ed25519, AES-GCM, JWS, JAdES, SD-JWT, encrypted keystore
    dpp-vc/             W3C VCs, did:web, status lists, LocalIdentityService, JSON-LD
    dpp-digital-link/   GS1 Digital Link parser, link-type negotiation
    dpp-aas/            Asset Administration Shell shells and submodels
    dpp-calc/           EU-methodology calculators (CO2e, repairability)
    dpp-plugin-traits/  Wasm plugin ABI
    dpp-plugin-sdk/     guest-side SDK: export_plugin! macro and Validator
    dpp-registry/       EU registry types (wasm32-safe)
    dpp-vocab/          external vocabulary register, one file per authority
    dpp-tests/          cross-crate integration tests and structural checks (not published)
  benches/              Criterion benchmarks
  plugins/              11 product group plugins (wasm32-wasip1, outside the workspace)
  docs/                 architecture, regulatory and governance documents
```

### Dependency Rules

The dependency graph is acyclic:

```
dpp-rules         -> (standalone, no internal deps; no_std, zero-dep)
dpp-plugin-traits -> (standalone, no internal deps)
dpp-vocab         -> (standalone, no internal deps)
dpp-crypto        -> (none)
dpp-domain        -> dpp-rules
dpp-vc            -> dpp-domain, dpp-crypto, dpp-vocab
dpp-digital-link  -> dpp-domain, dpp-rules
dpp-aas           -> dpp-domain, dpp-vocab
dpp-registry      -> dpp-domain, dpp-rules
dpp-calc          -> dpp-rules
dpp-plugin-sdk    -> dpp-plugin-traits, dpp-rules
dpp-tests         -> dpp-domain, dpp-crypto, dpp-vc
                     + dev: dpp-digital-link, dpp-aas, dpp-registry, dpp-vocab, dpp-rules
dpp-benches       -> dev only: dpp-domain, dpp-crypto, dpp-digital-link, dpp-aas, dpp-calc, dpp-tests
```

`crates/dpp-tests/tests/crate_graph.rs` parses the block above and compares it with
the manifests, so update both together.

`dpp-rules` is the single home for identifier syntax (GTIN check digits, GS1 CSET 82).
Anything that must agree with the domain about whether an identifier is well formed
depends on it directly instead of restating the table. Because `dpp-rules` is `no_std`
with no dependencies, that edge costs nothing.

`dpp-tests` keeps three edges as normal dependencies because `src/fixtures.rs` is a
library used by the integration tests. The crate is `publish = false`, so none of these
edges reach a published graph.

**No crate's `[dependencies]` may name** axum, tokio, tower, sqlx, redis, reqwest, or
any other HTTP, database or infrastructure crate. A dependency that brings in an async
runtime or network I/O does not belong in this workspace.

The one exception is a test runtime in `[dev-dependencies]`: `tokio` with `rt` and
`macros` in `dpp-crypto`, `dpp-domain`, `dpp-vc` and `dpp-tests`, so `#[tokio::test]`
can drive the async port traits. It is never compiled into a published crate.

Plugins are held to a stricter rule. A plugin is deterministic and does no I/O, and it
runs inside a host as Wasm, so none of those crates may appear anywhere in
`plugins/Cargo.lock`, directly or transitively, not even `tokio` for tests.

## 4. Code conventions

### Where a file goes

[`docs/architecture/CODE-LAYOUT.md`](docs/architecture/CODE-LAYOUT.md) is the standard:
one public type per file, `mod.rs` is a pure index, tests live in sibling files rather
than inline, every file opens with a `//!` doc comment, and only `lib.rs` sits at a
crate's `src/` root.

Tests in `crates/dpp-tests/tests/` enforce most of these rules, so `just check` reports
a violation before a reviewer does. Each test carries a baseline of files that already
violate it. **Do not add a file to a baseline to make the check pass.** Fix the file, or
mark it with a `// LAYOUT-DEVIATION: <reason>` comment that states why.

### No I/O in library code

Every crate compiles without I/O crates. Code that imports `axum`, `sqlx` or
`async-nats` belongs in the application that uses the library.

`KeyStore` in `dpp-crypto` reads and writes its key file with `std::fs`. That is
configuration I/O (a local file the operator chose), not a call to a database or a
network service, and it is the only exception.

### Errors

Library crates define typed error enums with `thiserror`, so callers can match on
variants. No `String` errors in domain code.

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

All crates use `tracing`. API keys, private keys and passphrases never appear in log
output.

### Lints and formatting

Code passes `cargo clippy --workspace --all-targets --all-features -- -D warnings` and
`cargo fmt --all --check`.

## 5. Tests

| Kind | Where | Scope |
|---|---|---|
| Unit tests | a sibling `tests.rs` (or `<name>_tests.rs`) next to the code | Pure logic |
| Crate integration tests | `crates/<crate>/tests/` | One crate's public API |
| Cross-crate and structural tests | `crates/dpp-tests/tests/` | Behaviour across crates, layout rules, documentation drift |
| Doctests | each crate's README and doc comments | The examples readers copy |

No test needs Docker, a database or network access.

## 6. Changing a schema

A published schema version is never edited: a passport validated against it must stay
valid against it. Every change is a new version.

To add a version to an existing product group:

1. Add `crates/dpp-domain/schemas/<product-group>/v<version>.json`.
2. Add an `EmbeddedSchema` entry for it in `crates/dpp-domain/src/schemas/embedded.rs`.
3. If it becomes the current version, update `currentSchemaVersion` in
   `crates/dpp-domain/product-groups/<product-group>.json`.
4. Run `just schema-changes` to regenerate
   [`docs/architecture/SCHEMA-CHANGES.md`](docs/architecture/SCHEMA-CHANGES.md).
5. If the change is not purely additive, read
   [`docs/architecture/PERSISTED-SHAPES.md`](docs/architecture/PERSISTED-SHAPES.md)
   first. Stored passports must still deserialise.

`just check` reports any document or table that has drifted from the schemas.

## 7. Commits

[Conventional Commits](https://www.conventionalcommits.org/) v1.0.0:

```text
<type>(<scope>): <subject>
```

- **Types:** `feat`, `fix`, `docs`, `refactor`, `test`, `perf`, `chore`, `ci`, `build`, `style`, `revert`. CI rejects any other type.
- **Scopes:** the crate or area touched, for example `domain`, `rules`, `crypto`, `vc`,
  `digital-link`, `aas`, `calc`, `registry`, `vocab`, `schemas`, `plugins`, `deps`,
  `ci`, `docs`. Never the repository's own name.
- **Subject:** imperative mood, under 50 characters.
- **Breaking change:** `!` after the scope, for example
  `feat(domain)!: remove deprecated field from Passport`.

```text
feat(crypto): verify JWS against an archived key
fix(domain): handle an empty product group in VersionedSchemaRegistry
docs(schemas): add textile v1.5.0
chore(deps): upgrade ed25519-dalek to 2.2.1
```

### Sign-off

Sign off every commit under the
[Developer Certificate of Origin v1.1](https://developercertificate.org/). The
`Signed-off-by:` line certifies that you wrote the contribution, or have the right to
submit it under Apache-2.0.

```bash
git commit -s -m "feat(domain): add a port trait"
```

## 8. Pull requests

- Every change goes through a pull request. Nobody pushes directly to `main`.
- CI passes: build, tests, clippy, audit, format, Wasm builds.
- A change to a port trait explains its effect on implementers.
- A new dependency is justified in the description.
- The pull request title is a Conventional Commit.
- Branch names follow the type: `feat/schema-registry-versioning`,
  `fix/crypto-key-rotation-archive`, `docs/wasm-target-guide`.

## 9. Security practices

- **No hard-coded secrets**, including in tests. Use temporary files with random names.
- **No `unsafe` code** without a recorded architecture decision.
- **No `unwrap()` or `expect()` on library code paths.** Tests and examples may use them.
- **`cargo audit` passes** in CI.

Report vulnerabilities privately to **security@odal-node.io**, never in a public issue.
See [SECURITY.md](SECURITY.md).
