# Release Process

This document describes how dpp-core releases are prepared, validated, and
published to crates.io.

## Tooling

- [`cargo-release`](https://github.com/crate-ci/cargo-release) — automates
  version bumps, git tags, and crates.io publishing.
- [`cargo-semver-checks`](https://github.com/obi1kenobi/cargo-semver-checks) —
  detects unintentional breaking changes before release.
- [`cargo-audit`](https://rustsec.org/) — checks dependencies against the
  RustSec advisory database.

## Release Cadence

There is no fixed schedule. Releases are cut when a meaningful set of changes
has accumulated and all checks pass. Patch releases for security fixes are
published as soon as the fix is verified.

## Pre-Release Checklist

Before running `cargo release`:

1. **All CI green** — `just check` passes locally (fmt, clippy, nextest, audit).
2. **CHANGELOG.md updated** — move items from `[Unreleased]` to a new version
   heading with today's date. Follow Keep a Changelog format.
3. **cargo-semver-checks clean** — run `cargo semver-checks` against the
   previous release tag. If violations are found, adjust the version bump
   level or fix the API.
4. **No `TODO` or `FIXME` in public API** — `grep -rn 'TODO\|FIXME' crates/`
   should return nothing in public-facing doc comments.
5. **Doc-tests pass** — `cargo test --doc --workspace`.
6. **README accuracy** — verify that the root README and each crate README
   reflect the current API and feature set.
7. **Dependency review** — check that no new dependencies introduce
   problematic licenses. All dependencies must be compatible with Apache-2.0.

## Publishing Order

Because of inter-crate dependencies, crates must be published in topological
order:

1. `dpp-rules` (no workspace dependencies)
2. `dpp-plugin-traits` (no workspace dependencies)
3. `dpp-domain` (depends on dpp-rules)
4. `dpp-registry` (depends on dpp-domain)
5. `dpp-digital-link` (depends on dpp-domain)
6. `dpp-crypto` (depends on dpp-domain)
7. `dpp-calc` (depends on dpp-domain)
8. `dpp-plugin-sdk` (depends on dpp-plugin-traits + dpp-rules)

`dpp-tests` is `publish = false` and is not published. `cargo-release` handles
this ordering automatically when run from the workspace root with `--workspace`.

## Release Command

```sh
# Dry run first — always
cargo release patch --workspace --dry-run

# If the dry run is clean
cargo release patch --workspace --execute
```

Replace `patch` with `minor` or `major` as appropriate per the
[versioning policy](VERSIONING.md).

## Post-Release

1. Verify the crates appear on [crates.io](https://crates.io/) and that
   docs.rs builds succeed.
2. Create a GitHub Release from the generated git tag with a summary pulled
   from CHANGELOG.md.
3. Announce the release in the project's communication channels.

## Yanking a Release

If a published version has a critical defect:

1. `cargo yank --version <ver> <crate>` for each affected crate.
2. Publish a patch release with the fix.
3. Add a **Yanked** note to CHANGELOG.md explaining why.

Yanking is a last resort. Prefer publishing a patch release whenever possible,
since yanking breaks downstream `Cargo.lock` files.

## Sector Plugins

Wasm sector plugins (`plugins/sector-*`) are not part of the workspace and are
not published to crates.io. They are released as `.wasm` artefacts attached to
GitHub Releases. Their versions track independently from the workspace version.

## References

- [cargo-release documentation](https://github.com/crate-ci/cargo-release)
- [crates.io publishing guide](https://doc.rust-lang.org/cargo/reference/publishing.html)
- [Versioning policy](VERSIONING.md)
