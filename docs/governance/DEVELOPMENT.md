# Development Guide

This document covers the development workflow, branching strategy, CI
requirements, and code review expectations for dpp-core.

## Prerequisites

- **Rust** — stable toolchain (see `rust-version` in root `Cargo.toml` for MSRV).
- **just** — command runner ([casey/just](https://github.com/casey/just)).
- **cargo-nextest** — test runner (`cargo install cargo-nextest`).
- **wasm32-wasip1 target** — only needed for plugin builds
  (`rustup target add wasm32-wasip1`).

Optional but recommended:

- `cargo-semver-checks` — API compatibility checking.
- `cargo-audit` — dependency vulnerability scanning.
- `cargo-release` — release automation.

## Building

```sh
just build          # Release build, all workspace crates
just build-plugins  # Compile Wasm sector plugins (wasm32-wasip1)
```

The build must succeed with **zero infrastructure running** — no database, no
Redis, no environment variables. This is the golden rule of dpp-core: it is a
pure domain library.

## Testing

```sh
just test   # Runs cargo nextest run --workspace
```

All tests are deterministic and require no I/O. Tests that need randomness use
seeded RNGs. The full suite should complete in under 30 seconds.

## Quality Gate

Before pushing, run the full gate:

```sh
just check  # fmt-check → lint → test → audit
```

This is the same sequence CI runs. A PR will not be merged unless `just check`
passes.

Individual steps:

| Command | What it does |
|---|---|
| `just fmt` | Format all code with `rustfmt` |
| `just lint` | `cargo clippy --workspace --all-targets -- -D warnings` |
| `just test` | `cargo nextest run --workspace` |
| `cargo audit` | Check for known vulnerabilities in dependencies |

Run `just --list` for the full recipe menu (`build`, `build-plugins`, `bench`, `doc`, …).

## Branching Strategy

Trunk-based: short-lived `feat/*` / `fix/*` / `chore/*` branches merge to `main`
via PR; no long-lived branches; tags are cut from `main`. Branch protection,
commit conventions, and release tagging are detailed in
[GIT-STRATEGY.md](GIT-STRATEGY.md) — not repeated here.

## Pull Request Process

1. **Open an issue first** for non-trivial changes. This avoids wasted effort
   on changes that conflict with the project direction.
2. **One logical change per PR.** Split unrelated changes into separate PRs.
3. **DCO sign-off required.** Every commit must include a `Signed-off-by:`
   line. Use `git commit -s` to add it automatically.
4. **CI must pass.** The PR cannot be merged until all status checks are green.
5. **Respond to review feedback** or explain why you disagree. Silence is not
   consent — unresolved threads block merging.

### PR Title Convention

Use [Conventional Commits](https://www.conventionalcommits.org/) prefixes in
the PR title:

- `feat:` — new functionality
- `fix:` — bug fix
- `docs:` — documentation only
- `refactor:` — code restructuring, no behaviour change
- `test:` — adding or updating tests
- `chore:` — build, CI, tooling changes
- `breaking:` — API-breaking change (also requires CHANGELOG entry)

### Review Expectations

- The maintainer reviews all PRs.
- Reviewers check for correctness, clarity, test coverage, and adherence to
  the project's architectural patterns (hexagonal architecture, port traits).
- Nit-level feedback (formatting, naming) is prefixed with `nit:` and is
  non-blocking.
- Security-sensitive changes require explicit security-focused review (see
  [GOVERNANCE.md](GOVERNANCE.md)).

## Code Style

- Follow `rustfmt` defaults. Do not override `rustfmt.toml` without
  discussion.
- Prefer explicit types over `impl Trait` in public API signatures.
- Public items must have doc comments (`///`). The `missing_docs` lint will be
  enabled before 1.0.
- Error types use `thiserror` for derivation. Do not use `anyhow` in library
  code.
- No `unwrap()` or `expect()` in library code. Use `Result` propagation.
- No `unsafe` unless absolutely necessary, with a `// SAFETY:` comment
  explaining the invariant.

## Adding a New Schema

1. Create `schemas/{sector}/v{N}.json` with the JSON Schema.
2. Register it in `VersionedSchemaRegistry` (the `include_str!()` macro
   embeds it at compile time).
3. Add a test in `dpp-domain` that validates a sample document against the
   new schema.
4. Update CHANGELOG.md under `[Unreleased]`.

## Adding a New Port Trait

1. Define the trait in `dpp-domain::ports`.
2. Add it to the re-exports in `dpp-domain::lib.rs`.
3. Document the trait's contract, error semantics, and concurrency
   expectations.
4. Do NOT provide an implementation — implementations live in the platform
   repo.
5. Update the architecture docs if the new port changes the core/platform
   boundary.

## References

- [just command runner](https://github.com/casey/just)
- [cargo-nextest](https://nexte.st/)
- [Conventional Commits](https://www.conventionalcommits.org/)
- [Versioning policy](VERSIONING.md)
- [Release process](RELEASE.md)
