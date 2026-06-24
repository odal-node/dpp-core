# dpp-benches

Criterion micro-benchmarks for the `dpp-core` hot paths — the operations that run
on every passport publish, export, or verification. Not published (`publish = false`),
excluded from the public API surface; this crate exists purely to track the
performance of the kernels that matter.

## Running

```bash
just bench                          # all benchmarks (cargo bench --package dpp-benches)
cargo bench --package dpp-benches   # equivalent
cargo bench -p dpp-benches crypto   # a single bench target
```

Criterion writes HTML reports and regression baselines to `target/criterion/`.
Re-running compares against the previous baseline, so you can see whether a change
moved a number.

## Benchmark Targets

| Target | Source | What it measures |
|---|---|---|
| `crypto` | [src/crypto.rs](src/crypto.rs) | Ed25519 + JWS: signing a payload, verifying via the keystore, and standalone verification against a raw public key |
| `validation` | [src/validation.rs](src/validation.rs) | JSON Schema sector validation — single battery, single textile, and a 100-record mixed batch (validators are warmed first so schema compilation isn't measured) |
| `gs1` | [src/gs1.rs](src/gs1.rs) | GS1 Digital Link parsing (AI 01/21 and full 01/10/21 URIs) |
| `calc` | [src/calc.rs](src/calc.rs) | EU-methodology calculators: cradle-to-gate CO₂e (small + 50-material bill) and the simplified repairability heuristic |
| `aas` | [src/aas.rs](src/aas.rs) | AAS submodel mapping (`build_aas_from_passport`) and the build-and-serialise path used on Catena-X / IDTA export |

## Conventions

- Each target is a standalone Criterion binary (`harness = false`) registered as a
  `[[bench]]` in [Cargo.toml](Cargo.toml).
- Setup work (key generation, validator warm-up, fixture construction) is done
  **outside** the `b.iter(...)` closure so only the operation under test is timed.
- Fixtures use realistic sector data (a battery passport with Annex VII fields, a
  60/40 cotton/polyester textile) rather than minimal stubs, so the numbers reflect
  production-shaped inputs.

When you add a hot path to `dpp-core`, add a bench target here: a new `src/<name>.rs`
plus a `[[bench]]` entry in [Cargo.toml](Cargo.toml).
