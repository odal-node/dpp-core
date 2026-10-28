# Versioning policy

How `dpp-core` versions its crates, what stability each version range provides, and how
breaking changes are communicated.

## Scheme

`dpp-core` follows [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html):

```text
MAJOR.MINOR.PATCH
```

- **MAJOR:** incompatible API changes.
- **MINOR:** backwards-compatible new functionality.
- **PATCH:** backwards-compatible bug fixes.

## Before 1.0

While a crate is below 1.0.0:

- A **minor** bump (0.x.0 to 0.y.0) may contain breaking changes. Each one is listed in
  CHANGELOG.md under a **Breaking** heading with a migration note.
- A **patch** bump (0.x.y to 0.x.z) is always backwards-compatible.
- Items marked `#[doc(hidden)]` or behind a feature flag named `unstable-*` carry no
  stability guarantee.

The aim is to reach 1.0.0 once the EU delegated acts for the first product group
(batteries) are final and the public API has been validated against real integrations.
This is a project target, not a legal deadline. The date the battery passport applies
from is in the instrument catalog in `dpp-domain`.

## One version for the whole workspace

Every workspace crate except `dpp-benches` shares one version, set in the root
`Cargo.toml` through `workspace.package.version`, so every release bumps each of them.
`dpp-benches` is unpublished and keeps its own `0.0.0`. The reasons:

1. The crates that depend on each other are tightly coupled, and `dpp-domain` is their
   hub: `dpp-aas`, `dpp-digital-link`, `dpp-registry`, `dpp-vc` and `dpp-tests` all build on
   it. (`dpp-rules`, `dpp-crypto`, `dpp-calc`, `dpp-vocab`, `dpp-plugin-traits` and
   `dpp-plugin-sdk` do not, so lockstep bumps them for consistency rather than necessity.)
2. One version makes compatible combinations obvious to downstream users.
3. Once crates stabilise at different rates after 1.0, lockstep may be relaxed. That
   decision will be recorded here and in the changelog.

The Wasm product group plugins are outside the workspace but inside the lockstep:
`plugins/Cargo.toml` carries the same version, and a test fails when it does not. See
[`RELEASE.md`](RELEASE.md#product-group-plugins) for why, and for the step this adds to a
version bump.

## Detecting breaking changes

[`cargo-semver-checks`](https://crates.io/crates/cargo-semver-checks) runs twice, for
two different questions. Neither run fails the build: below 1.0 a minor release is allowed
to break, and a check that is red by design is one nobody reads.

- **On every pull request**, `.github/workflows/semver.yml` compares the change with the
  branch it merges into and prints what it breaks in the job summary. It is a report. It
  does not see a changed function parameter type, so a clean report is a lower bound, not
  the answer.
- **At release**, `just semver` compares the workspace with the last published release.
  Its list has to match CHANGELOG's `### Breaking` section ([`RELEASE.md`](RELEASE.md),
  step 3), and that is where a break is reconciled with the version number.

When the pull request report names a break, the author and reviewer read it, and the
author either:

1. adjusts the change to stay backwards-compatible, or
2. adds a **Breaking** entry to CHANGELOG.md. The version moves with the release.

## Deprecation

1. Mark the item with `#[deprecated(since = "0.x.0", note = "Use Y instead")]`.
2. Add a note to CHANGELOG.md under **Deprecated**.
3. Remove the item no earlier than the next minor release.
4. Before 1.0, a deprecation may last a single minor cycle.

## Rust edition and MSRV

- **Edition:** 2024, set in the root `Cargo.toml` through `workspace.package.edition`.
- **MSRV:** `rust-version` in the root `Cargo.toml`. Raising it is a minor-version change.

## Schema versions

The JSON Schemas under `crates/dpp-domain/schemas/{product-group}/v{N}.json` are versioned independently of
the crates. Adding a schema version is always a minor crate change. Removing or altering an
existing schema version is a breaking change. See
[COMPLIANCE.md](../regulatory/COMPLIANCE.md) for the mapping from schemas to regulations.

## References

- [SemVer 2.0.0](https://semver.org/spec/v2.0.0.html)
- [Cargo SemVer compatibility](https://doc.rust-lang.org/cargo/reference/semver.html)
- [cargo-semver-checks](https://crates.io/crates/cargo-semver-checks)
- [cargo-release](https://crates.io/crates/cargo-release)
