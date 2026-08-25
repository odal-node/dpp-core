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
3. **Breaking changes reconciled** — run `just semver`. It lists every public
   API removal since the last published release; **every entry must already
   appear under `### Breaking` in the CHANGELOG**. An entry that does not is
   the finding: a break nobody recorded, which is how 0.13.0 withdrew public
   constants unannounced.

   Do **not** read a red result as "adjust the version bump level". Under the
   pre-1.0 conventions in [`VERSIONING.md`](VERSIONING.md) a minor bump already
   admits breaking changes, so the bump level is never the answer — which is
   also why the recipe passes `--release-type patch`. Without it,
   `cargo semver-checks` sees a minor bump, waives all 253 lints and reports
   "no semver update required" having checked nothing. Measured at
   0.16.0 → 0.17.0 on this workspace: 0 checks run.
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
3. `dpp-crypto` (no workspace dependencies)
4. `dpp-calc` (no workspace dependencies)
5. `dpp-vocab` (no workspace dependencies — a leaf, by design)
6. `dpp-domain` (depends on dpp-rules)
7. `dpp-registry` (depends on dpp-domain)
8. `dpp-digital-link` (depends on dpp-domain)
9. `dpp-aas` (depends on dpp-domain + dpp-vocab)
10. `dpp-vc` (depends on dpp-domain + dpp-crypto)
11. `dpp-plugin-sdk` (depends on dpp-plugin-traits + dpp-rules)

`dpp-vocab` was first published in 0.17.0. Until then `just semver` excluded it,
because a crate with no crates.io baseline has nothing to diff against; that
exclusion has been removed. **A new crate needs the same treatment exactly
once** — exclude it for its first release, then put it back. Leaving it out is
how a crate ends up with nobody checking its public API.

`dpp-tests` is `publish = false` and is not published, but it is still lockstep-
versioned with everything else. `cargo-release` handles publish ordering
automatically when run from the workspace root with `--workspace`.

`dpp-benches` is deliberately pinned at `version = "0.0.0"`, outside lockstep
(see [`VERSIONING.md`](VERSIONING.md)) — it must always be excluded or
`cargo-release` will try to bump it too.

`[workspace.metadata.release]` in the root `Cargo.toml` pins the tag scheme to
one annotated `vX.Y.Z` tag with message `Release version X.Y.Z` for the whole
release, instead of cargo-release's default of one tag per publishable crate.

## Release Command

**Check `Cargo.toml` first.** In this project the version bump usually rides in
on a feature PR rather than being made at release time, so by the time you get
here `[workspace.package] version` is often *already* the version you intend to
ship. Passing a bump level in that state releases the version **after** the one
you meant:

```sh
# The version in Cargo.toml is already the one you are shipping — the
# usual case here. No level argument: publish and tag what is there.
cargo release --workspace --exclude dpp-benches --dry-run
cargo release --workspace --exclude dpp-benches --execute
```

```sh
# The version in Cargo.toml is still the last *released* one, and you want
# cargo-release to bump it for you.
cargo release minor --workspace --exclude dpp-benches --dry-run
cargo release minor --workspace --exclude dpp-benches --execute
```

Replace `minor` with `patch` or `major` as appropriate per the
[versioning policy](VERSIONING.md). `--execute` also pushes the commit and tag
to `origin` unless `--no-push` is given.

Read the dry run's output rather than skimming it: it names the exact version
each crate will be published at. That line is the check on which of the two
cases above you are actually in.

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

## Product group Plugins

Wasm product group plugins (`plugins/product-group-*`) are not part of the workspace and are
not published to crates.io. They are released as `.wasm` artefacts attached to
GitHub Releases. Their versions track independently from the workspace version.

## References

- [cargo-release documentation](https://github.com/crate-ci/cargo-release)
- [crates.io publishing guide](https://doc.rust-lang.org/cargo/reference/publishing.html)
- [Versioning policy](VERSIONING.md)
