# Versioning Policy

This document describes how dpp-core versions its crates, what stability
guarantees each version range provides, and how breaking changes are
communicated.

## Scheme

dpp-core follows [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html):

    MAJOR.MINOR.PATCH

- **MAJOR** — incompatible API changes.
- **MINOR** — backwards-compatible new functionality.
- **PATCH** — backwards-compatible bug fixes.

## Pre-1.0 Conventions

While any crate is below 1.0.0 the following rules apply:

- A **minor** bump (0.x.0 -> 0.y.0) may contain breaking changes. Each such
  change is listed in CHANGELOG.md under a **Breaking** heading with a
  migration note.
- A **patch** bump (0.x.y -> 0.x.z) is always backwards-compatible.
- There is no stability guarantee on items marked `#[doc(hidden)]` or gated
  behind a feature flag named `unstable-*`.

The goal is to reach 1.0.0 for each crate once the EU ESPR delegated acts
for the first sector (batteries, February 2027) are finalised and the public
API has been validated against real-world integrations.

## Workspace Version Lockstep

All workspace crates share a single version number defined in the root
`Cargo.toml` via `workspace.package.version`. This means every release bumps
all crates together. The rationale:

1. The crates are tightly coupled — `dpp-domain` is a dependency of every
   other crate.
2. A single version makes it trivial for downstream consumers to ensure
   compatible combinations.
3. Once individual crates stabilise at different rates (post-1.0), lockstep
   may be relaxed. That decision will be recorded in this document and the changelog.

## Breaking Change Detection

CI runs [`cargo-semver-checks`](https://github.com/obi1kenobi/cargo-semver-checks)
on every pull request targeting `main`. A detected semver violation fails the
check and requires the author to either:

1. Adjust the change to be backwards-compatible, or
2. Bump the version accordingly and add a **Breaking** entry to CHANGELOG.md.

## Deprecation Process

1. Mark the item with `#[deprecated(since = "0.x.0", note = "Use Y instead")]`.
2. Add a note to CHANGELOG.md under **Deprecated**.
3. The deprecated item is removed no earlier than the next minor release.
4. For pre-1.0 crates, deprecation may last only one minor cycle.

## Rust Edition and MSRV

- **Rust edition**: 2024 (set in the root `Cargo.toml` via `workspace.package.edition`).
- **MSRV (Minimum Supported Rust Version)**: documented in the root
  `Cargo.toml` under `rust-version`. Bumping MSRV is a minor-version change.

## Schema Versioning

JSON schemas under `schemas/{sector}/v{N}.json` are versioned independently
from crate versions. Adding a new schema version is always a minor crate
change. Removing or altering an existing schema version is a breaking change.
See [COMPLIANCE.md](../regulatory/COMPLIANCE.md) for details on the schema-to-regulation
mapping.

## References

- [SemVer 2.0.0 specification](https://semver.org/spec/v2.0.0.html)
- [Cargo SemVer compatibility](https://doc.rust-lang.org/cargo/reference/semver.html)
- [cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks)
- [cargo-release](https://github.com/crate-ci/cargo-release)
