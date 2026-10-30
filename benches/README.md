# dpp-benches

Criterion micro-benchmarks for the `dpp-core` operations that run on every passport
publish, export, read or verification. Not published (`publish = false`) and not part of the
public API.

## Run

```bash
just bench                          # all benchmarks (cargo bench --package dpp-benches)
cargo bench --package dpp-benches   # the same
cargo bench -p dpp-benches crypto   # one bench target
cargo bench -p dpp-benches --bench access -- --test   # run each benchmark once, as a check
```

Criterion writes HTML reports and regression baselines to `target/criterion/`. Each run
compares against the previous baseline, so you can see whether a change moved a number.

## Targets

| Target | Source | Measures |
|---|---|---|
| `crypto` | [src/crypto.rs](src/crypto.rs) | Ed25519 and JWS: signing and verifying a small payload, then whole passports of 1, 50 and 500 materials; and `KeyStore::open`, whose Argon2id cost is a security parameter, not something to drive down |
| `canonical` | [src/canonical.rs](src/canonical.rs) | RFC 8785 canonicalisation, the byte form every signature covers: a small payload and whole passports |
| `validation` | [src/validation.rs](src/validation.rs) | JSON Schema validation: a battery, a textile and a 100-record mixed batch; batches of 10, 100 and 1000; the whole-passport publish gate; and every product group's schema against its frozen compatibility fixture |
| `access` | [src/access.rs](src/access.rs) | `redact_passport`, the audience filter every served view goes through: per audience, the fail-closed path, and growth with the bill of materials |
| `passport_serde` | [src/passport_serde.rs](src/passport_serde.rs) | `Passport` to and from JSON text and `Value`, at 1 and 500 materials |
| `sd_jwt` | [src/sd_jwt.rs](src/sd_jwt.rs) | SD-JWT concealment, the wire format, presenting half the claims, and issuing and verifying a battery passport as an SD-JWT VC |
| `status_list` | [src/status_list.rs](src/status_list.rs) | Bitstring Status List: decoding a published list of 131072 and 1048576 entries, and testing one bit |
| `linktype` | [src/linktype.rs](src/linktype.rs) | Digital Link content negotiation over 5 and 50 representations, and `Accept` header parsing |
| `gs1` | [src/gs1.rs](src/gs1.rs) | GS1 Digital Link parsing (the bare key, a query string, a path prefix, an escaped value, the two refusals), building a link back out, and minting a carrier URL from a passport |
| `calc` | [src/calc.rs](src/calc.rs) | Cradle-to-gate CO2e (2 and 50 materials, and a ladder up to 10000) and the simplified repairability heuristic |
| `aas` | [src/aas.rs](src/aas.rs) | AAS submodel mapping (`build_aas_from_passport`) and the build-and-serialise path used for Catena-X and IDTA export, per audience and across bills of materials |

## Conventions

- Each target is a standalone Criterion binary (`harness = false`) registered as a
  `[[bench]]` in [Cargo.toml](Cargo.toml).
- Setup (key generation, validator warm-up, fixtures) happens **outside** the
  `b.iter(...)` closure, so only the operation under test is timed. An operation that
  consumes its input is given a copy by `iter_batched`, not by cloning inside the timed
  closure.
- Fixtures use realistic product group data (a battery passport with Annex XIII fields,
  a 60/40 cotton and polyester textile), so the numbers reflect production-shaped
  inputs. They are shared through [src/fixtures.rs](src/fixtures.rs), which each target
  includes with `#[path = "fixtures.rs"] mod fixtures;`.
- **A fixture asserts it reaches the path its benchmark is named for.** A benchmark that
  times a failure, a fail-closed fallback or an empty result still prints a figure.
  Where that is possible, the setup checks before timing: a passport passes the publish
  gate, a negotiation finds what it is meant to find, a view is narrower for the public
  than for a credentialed reader. Run `-- --test` after changing a fixture.
- **Existing benchmark names do not change.** Criterion compares by name, so a rename
  discards the baseline. A sweep adds new names beside the old ones.
- Sweeps report throughput where the work is proportional to the input (bytes, rows or
  materials), so a change in how a cost grows shows as a change in slope.

When you add a hot path to `dpp-core`, add a bench target: a new `src/<name>.rs` and a
`[[bench]]` entry in [Cargo.toml](Cargo.toml).

## In CI

[`bench.yml`](../.github/workflows/bench.yml) runs everything weekly and on demand. It
is a measurement, not a gate: a shared runner is too noisy to fail a build on a timing.
It restores the previous run's `target/criterion` from a cache, so Criterion's own
comparison is against the last run, and it writes the benches Criterion judges slower to
the run summary. The two runs may not be on alike machines, so a flagged bench is a
reason to run it again, not a finding.
