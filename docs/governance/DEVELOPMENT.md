# Development guide

The development workflow, CI requirements and code review expectations for `dpp-core`.
[CONTRIBUTING.md](../../CONTRIBUTING.md) is the entry point; this document adds detail.

## Prerequisites

- **Rust:** the toolchain pinned in `rust-toolchain.toml` (the MSRV is `rust-version` in
  the root `Cargo.toml`).
- **just:** the command runner ([casey/just](https://github.com/casey/just)).
- **cargo-nextest:** the test runner (`cargo install cargo-nextest`).
- **`wasm32-wasip1` target:** only for plugin builds (`rustup target add wasm32-wasip1`).

Optional:

- `cargo-semver-checks` for API compatibility checks.
- `cargo-audit` for dependency advisories.
- `cargo-release` for releases.

## Building

```bash
just build          # release build of every workspace crate
just build-plugins  # build the Wasm product group plugins (wasm32-wasip1)
```

The build must succeed with **nothing running**: no database, no Redis, no environment
variables. This is a library with no infrastructure.

## Testing

```bash
just test   # cargo nextest run --workspace
```

Tests need no network and no services.

## Quality gate

Before pushing, run:

```bash
just check
```

It runs formatting, clippy (workspace and plugins), tests, doctests, plugin tests, the
plugin gate self-test, the doc build, the lockfile check and `cargo audit`. CI runs the
same steps as separate jobs and adds the Wasm cross-compiles and the oracle suites, so
`just ci` is the closer match before a push. A pull request is not merged unless CI passes.

Individual steps:

| Command | What it does |
|---|---|
| `just fmt` | Format all code with `rustfmt` |
| `just lint` | `cargo clippy --workspace --all-targets --all-features -- -D warnings` |
| `just test` | `cargo nextest run --workspace` |
| `just audit` | Check dependencies for known vulnerabilities |

`just --list` shows every recipe.

## Branching

Trunk-based: short-lived `feat/*`, `fix/*` and `chore/*` branches merge to `main` through a
pull request; no long-lived branches; tags are cut from `main`. Branch protection, commit
conventions and release tagging are in [GIT-STRATEGY.md](GIT-STRATEGY.md).

## Pull request process

1. **Open an issue first** for anything non-trivial, so effort is not spent on a change
   that conflicts with the project's direction.
2. **One logical change per pull request.** Split unrelated changes.
3. **Sign off every commit** with a `Signed-off-by:` line (`git commit -s`).
4. **CI passes** before merge.
5. **Respond to review feedback**, or explain why you disagree. Unresolved threads block
   merging.

### Pull request titles

The title is a [Conventional Commit](https://www.conventionalcommits.org/), checked by
`.github/workflows/pr-title.yml`. Allowed types: `feat`, `fix`, `docs`, `refactor`, `test`,
`perf`, `chore`, `ci`, `build`, `style`, `revert`. Mark a breaking change with `!` after
the type or scope (`feat(domain)!: …`) and file the CHANGELOG entry under the breaking
heading. The scope is a functional area, never `core` or `dpp-core`.

### Review expectations

- The maintainer reviews every pull request.
- Reviews check correctness, clarity, test coverage, and the project's architectural
  patterns (ports and adapters, port traits).
- Minor feedback (formatting, naming) is prefixed `nit:` and does not block.
- Security-sensitive changes need a review focused on security (see
  [GOVERNANCE.md](../../GOVERNANCE.md)).

## Code style

- `rustfmt` defaults. Do not change `rustfmt.toml` without discussion.
- Prefer explicit types over `impl Trait` in public API signatures.
- Public items have doc comments (`///`). The `missing_docs` lint will be enabled before
  1.0.
- Library error types derive with `thiserror`. A few functions in `dpp-crypto` and `dpp-vc`
  still return `anyhow::Result`; new library code uses typed errors.
- No `unwrap()` or `expect()` on library code paths; propagate `Result`.
- No `unsafe` unless necessary, with a `// SAFETY:` comment stating the invariant.

## Adding a schema

Follow [CONTRIBUTING.md §6](../../CONTRIBUTING.md#6-changing-a-schema): a new file under
`crates/dpp-domain/schemas/`, an entry in the embedded table, the manifest's
`currentSchemaVersion` if it becomes current, `just schema-changes`, and a CHANGELOG entry
under `[Unreleased]`.

## Adding a port trait

1. Define the trait in `dpp_domain::ports`, in its own module.
2. Add the module to the machine block in [PORTS.md](../architecture/PORTS.md);
   `ports_inventory.rs` fails until you do.
3. Document the trait's contract, error semantics and concurrency expectations.
4. Ship no production implementation: implementations live in the application. A stub for
   development may go under `ports::ghosts`, marked as a placeholder in its output.
5. Update the architecture documents if the port changes the boundary between the library
   and its hosts.

## References

- [just](https://github.com/casey/just)
- [cargo-nextest](https://nexte.st/)
- [Conventional Commits](https://www.conventionalcommits.org/)
- [Versioning policy](VERSIONING.md)
- [Release process](RELEASE.md)
