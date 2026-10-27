# Git strategy

**Model:** trunk-based development
**Remote:** `git@github.com:odal-node/dpp-core.git`
**Primary branch:** `main`

---

## Branches

```text
main         always releasable; releases are tagged here
  feat/*     features (short-lived)
  fix/*      bug fixes
  docs/*     documentation
  chore/*    CI, formatting, dependency bumps
```

No `develop` branch and no long-lived branches. Everything reaches `main` through a pull
request.

---

## Commits and pull request titles

[Conventional Commits](https://www.conventionalcommits.org/):

```text
<type>(<scope>): <subject>
```

- **Types** (checked on the pull request title by `.github/workflows/pr-title.yml`):
  `feat`, `fix`, `docs`, `refactor`, `test`, `perf`, `chore`, `ci`, `build`, `style`,
  `revert`.
- **Scope:** the functional area, usually the crate without its `dpp-` prefix: `domain`,
  `crypto`, `vc`, `digital-link`, `aas`, `rules`, `calc`, `registry`, `vocab`, `plugins`,
  `schemas`, `deps`, `ci`, `docs`. Several may be comma-separated. Never `core` or
  `dpp-core`; the check rejects them.
- **Breaking change:** `!` before the colon.

```text
feat(domain): add battery recycled-content disclosure fields
fix(digital-link): add a lifetime annotation to negotiate
test(crypto): audience gatekeeping integration tests
docs: update the README regulatory scope table
chore(deps): bump chrono to 0.4.39
ci: split unit and integration test steps
```

The pull request title becomes the squash commit subject on `main`, and cannot be
corrected after a release is published from that commit.

---

## Pull request flow

1. Create a branch: `git checkout -b feat/battery-soh-tracking`.
2. Commit in conventional format, signed off (`git commit -s`).
3. Push and open a pull request against `main`.
4. CI runs the required checks (below).
5. Squash-merge to `main`.
6. Delete the branch.

---

## Protection on `main`

Enforced by the `main` repository ruleset:

- Changes arrive only through a pull request; squash is the only merge method.
- The required status checks must pass on a branch that is up to date with `main`:
  format, clippy, tests, security audit, docs and doctests, product group plugin tests,
  the pull request title check, and the oracle suites (AAS, SD-JWT, DID, JSON-LD, GS1,
  JAdES).
- No force pushes and no deletion.

With one maintainer, no approving review is required; the checks are the gate.

Because the branch must be up to date before merging, merges are serial: after one pull
request merges, the next must be updated and its checks run again.

---

## Releases and tags

Tags are `vX.Y.Z`, one per release for the whole workspace, cut from `main` by
`cargo release` (see [RELEASE.md](RELEASE.md)). The `release tags` ruleset protects `v*`
tags from deletion, update and force push, so a published tag never moves.

---

## crates.io

Every crate except `dpp-tests` (`publish = false`) and `dpp-benches` is published to
crates.io in lockstep. [RELEASE.md](RELEASE.md) has the publishing order and the release
command; product group plugins are built as `.wasm` artefacts and never published to
crates.io.
