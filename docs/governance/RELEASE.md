# Release process

How `dpp-core` releases are prepared, checked and published to crates.io.

## Tooling

- [`cargo-release`](https://crates.io/crates/cargo-release): version bumps, git tags and
  crates.io publishing.
- [`cargo-semver-checks`](https://crates.io/crates/cargo-semver-checks): detects
  unintended breaking changes before release.
- [`cargo-audit`](https://rustsec.org/): checks dependencies against the RustSec advisory
  database.
- [`cargo-deny`](https://crates.io/crates/cargo-deny): checks dependency licences, bans and
  sources. `just check` runs it with `cargo-audit`, for every lockfile.

Install all four with
`cargo install cargo-release cargo-semver-checks cargo-audit cargo-deny`.

## Cadence

There is no fixed schedule. A release is cut when a meaningful set of changes has
accumulated and every check passes. A security fix is released as soon as it is verified.

## Pre-release checklist

Before running `cargo release`:

1. **CI is green:** the hosted CI run on the release commit passed. `just check` is the
   local preflight and does not replace it: it skips the Wasm cross-compiles, the plugin
   builds and the oracle suites.
2. **CHANGELOG.md is updated:** move the items from `[Unreleased]` to a new version heading
   with today's date, following Keep a Changelog.
3. **Breaking changes are reconciled:** run `just semver`. It lists every public API removal
   since the last published release, and **every entry must already appear under
   `### Breaking` in the CHANGELOG**. An entry that does not is the finding: a break nobody
   recorded, which is how 0.13.0 withdrew public constants without announcing it.

   Do **not** read a red result as "change the bump level". Under the pre-1.0 rules in
   [`VERSIONING.md`](VERSIONING.md) a minor bump already admits breaking changes, so the
   bump level is never the answer, which is also why the recipe passes
   `--release-type patch`. Without it, `cargo semver-checks` sees a minor bump, waives all
   253 lints and reports "no semver update required" having checked nothing. Measured at
   0.16.0 to 0.17.0 on this workspace: 0 checks run.
4. **No `TODO` or `FIXME` in public API docs:** `grep -rn 'TODO\|FIXME' crates/` returns
   nothing in public doc comments.
5. **Doctests pass:** `just test-doc` (`cargo test --doc --workspace --all-features`).
6. **READMEs are accurate:** the root README and each crate README match the current API
   and features.
7. **Dependencies reviewed:** no new dependency brings a problematic licence. Every
   dependency must be compatible with Apache-2.0.
8. **Standards re-read.** Re-read every row of
   [`docs/architecture/STANDARDS.md`](../architecture/STANDARDS.md) at its source and update
   its date:
   - RFCs at `https://www.rfc-editor.org/info/rfcNNNN`;
   - Internet-Drafts on the datatracker;
   - W3C specifications at `https://www.w3.org/TR/…`;
   - ETSI specifications in the `etsi.org/deliver` version listing;
   - IDTA and GS1 on their specification pages.

   A changed status is the finding. `standard_citations.rs` holds the IETF rows to the code,
   but it reads the register, not the IETF, so it cannot see an RFC become obsoleted. Run
   `just standards-status` first: it compares every IETF row with the RFC Editor's index and
   names the ones that differ, and it runs weekly between releases (`standards-status.yml`).
   That narrows the re-read for those rows without replacing it, since a row can be right
   about a status and stale about how the code uses the text. For the W3C, GS1, IDTA and
   ETSI rows, this re-read is the only check.

   The upstream vocabularies are read the same way. Run `just vocab-drift`: it checks every
   pin in `crates/dpp-vocab/vocabularies/` again and names the records whose source has
   moved. Read each named record again before the release, or say in its `finding` why
   not. It sees only what is pinned, which is why a test requires every `verified` record
   to pin something.
9. **Plugin version.** `plugins/Cargo.toml`'s `[workspace.package] version` must equal the
   workspace's, and the lockfiles of `plugins/` and `fuzz/` must agree (`just lock-check`).
   `cargo release` does both, through a replacement and a hook in
   `crates/dpp-tests/Cargo.toml`, because those are separate workspaces it does not
   otherwise see. Check that the dry run shows `Replacing in ../../plugins/Cargo.toml` with
   the version you mean to ship, and the `relock` hook's dry-run line. The lockfiles move
   only on `--execute`, where the release commit names `plugins/Cargo.toml`,
   `plugins/Cargo.lock` and `fuzz/Cargo.lock`. When the version bump comes in on a feature
   pull request, that pull request does the same by hand: edit the line and run
   `just relock`.

## Publishing order

Because of inter-crate dependencies, crates are published in topological order:

1. `dpp-rules` (no workspace dependencies)
2. `dpp-plugin-traits` (no workspace dependencies)
3. `dpp-crypto` (no workspace dependencies)
4. `dpp-calc` (depends on `dpp-rules`)
5. `dpp-vocab` (no workspace dependencies, by design)
6. `dpp-domain` (depends on `dpp-rules`)
7. `dpp-registry` (depends on `dpp-domain` and `dpp-rules`)
8. `dpp-digital-link` (depends on `dpp-domain` and `dpp-rules`)
9. `dpp-aas` (depends on `dpp-domain` and `dpp-vocab`)
10. `dpp-vc` (depends on `dpp-domain`, `dpp-crypto` and `dpp-vocab`)
11. `dpp-plugin-sdk` (depends on `dpp-plugin-traits` and `dpp-rules`)

`dpp-vocab` was first published in 0.17.0. Until then `just semver` excluded it, because a
crate with no crates.io baseline has nothing to compare against; that exclusion has been
removed. **A new crate needs the same treatment exactly once:** exclude it for its first
release, then put it back. Leaving it out means nobody checks its public API.

`dpp-tests` is `publish = false` and never published, but it shares the lockstep version.
`cargo-release` handles publishing order when run from the workspace root with
`--workspace`.

`dpp-benches` is pinned at `version = "0.0.0"`, outside lockstep (see
[`VERSIONING.md`](VERSIONING.md)), so it must always be excluded or `cargo-release` will try
to bump it.

`[workspace.metadata.release]` in the root `Cargo.toml` sets the tag scheme to one
annotated `vX.Y.Z` tag with the message `Release version X.Y.Z` for the whole release,
instead of cargo-release's default of one tag per publishable crate.

## Release command

**Check `Cargo.toml` first.** Here the version bump usually comes in on a feature pull
request rather than at release time, so by now `[workspace.package] version` is often
*already* the version you intend to ship. Passing a bump level in that state releases the
version **after** the one you meant:

```bash
# The version in Cargo.toml is already the one you are shipping (the usual case).
# No level argument: publish and tag what is there.
cargo release --workspace --exclude dpp-benches --dry-run
cargo release --workspace --exclude dpp-benches --execute
```

```bash
# The version in Cargo.toml is still the last released one, and cargo-release
# should bump it.
cargo release minor --workspace --exclude dpp-benches --dry-run
cargo release minor --workspace --exclude dpp-benches --execute
```

Replace `minor` with `patch` or `major` as the [versioning policy](VERSIONING.md) requires.
`--execute` also pushes the commit and tag to `origin` unless `--no-push` is given.

Read the dry run's output rather than skimming it: it names the exact version each crate
will be published at, which tells you which of the two cases you are in.

## After the release

1. Check that the crates appear on [crates.io](https://crates.io/) and that the docs.rs
   builds succeed.
2. Create a GitHub Release from the tag, with a summary taken from CHANGELOG.md.
3. Announce the release in the project's channels.

## Yanking a release

If a published version has a critical defect:

1. `cargo yank --version <ver> <crate>` for each affected crate.
2. Publish a patch release with the fix.
3. Add a **Yanked** note to CHANGELOG.md explaining why.

Yanking is a last resort. Prefer a patch release where possible, because yanking breaks
downstream `Cargo.lock` files.

## Product group plugins

The Wasm product group plugins (`plugins/product-group-*`) are outside the workspace and
are not published to crates.io, and no release ships their `.wasm` files. Their version and
lockfile are still part of every release:

**Their version is the workspace version, in lockstep.** `plugins/Cargo.toml` is a separate
workspace, so cargo does not move it. Whoever moves the workspace version moves
`[workspace.package] version` there to match, then runs `cargo update -w` in `plugins/`
(`just lock-check` fails until the lockfile agrees).
`crates/dpp-tests/tests/plugin_version_lockstep.rs` fails when the two differ.

This is not "bump the plugins when a plugin changes". A plugin changes whenever
`dpp-plugin-sdk`, `dpp-plugin-traits` or `dpp-rules` does, and none of those is under
`plugins/`: over the eleven releases from 0.11.0 to 0.21.0, `plugins/` itself changed in
four and that set of crates in seven. The version is an identity, not a compatibility
signal (a host negotiates on `AbiVersion` and `SchemaVersionRange`), so a plugin version
that moves when no plugin changed costs nothing, while one that stays put through a change
describes different artefacts with one string, as it did for nine releases.

## References

- [cargo-release](https://crates.io/crates/cargo-release)
- [Publishing on crates.io](https://doc.rust-lang.org/cargo/reference/publishing.html)
- [Versioning policy](VERSIONING.md)
