# dpp-tests

Cross-crate integration tests for Odal Node core. This crate contains
integration tests in `tests/` that exercise multiple core crates together
(`dpp-domain`, `dpp-crypto`, `dpp-digital-link`) and is intentionally not
published to crates.io (`publish = false`). Moving integration tests into a
real workspace member ensures they run as part of `cargo test --workspace` and
CI.

## When to use this crate

- To run the cross-crate integration test suite locally or in CI.
- If you need examples of how core crates interact (credential issuance,
  passport generation, digital link handling), check the tests in `tests/`.

## Run the tests

From the repository root run:

```bash
cargo test -p dpp-tests
```

Or run the full workspace test suite:

```bash
cargo test --workspace
```

## Relationship to other crates

| Crate | Role |
|---|---|
| `dpp-domain` | Provides domain objects used by the integration tests |
| `dpp-crypto` | Provides crypto primitives used in tests |
| `dpp-digital-link` | Used by tests that exercise GS1 / digital link interactions |

## Minimum Rust version

1.96 (MSRV is enforced in CI)

## License

Apache-2.0 — see [LICENSE](../../LICENSE)
