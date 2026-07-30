# dpp-aas

[![crates.io](https://img.shields.io/crates/v/dpp-aas.svg)](https://crates.io/crates/dpp-aas)
[![docs.rs](https://img.shields.io/docsrs/dpp-aas)](https://docs.rs/dpp-aas)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](../../LICENSE)

[Asset Administration Shell](https://industrialdigitaltwin.org/en/content-hub/aasspecifications)
(IDTA) projection of an [Odal Node](https://odal-node.io) Digital Product
Passport — shells and submodels for Industry 4.0 and Catena-X data spaces.

Pure Rust, no I/O, no network calls. `wasm32-unknown-unknown` safe.

⚠️ **This crate emits AAS-shaped output; it does not claim IDTA conformance.**
Producing a valid-looking Environment is not the same as conforming to a
specification, and nothing here should be described as IDTA-conformant.

## When to use this crate

- You need to render a typed `Passport` as an AAS shell plus submodels for an
  industrial data space.
- You are submitting to a registry that speaks AAS rather than raw DPP JSON.
- You want per-product-group submodels (battery, textile, electronics and eight
  more) rather than a flat key-value dump.

## Example

```rust
use dpp_aas::build_aas_from_passport;
use dpp_domain::Passport;

fn render_aas(passport: &Passport, gtin: &str) {
    let (shell, submodels) = build_aas_from_passport(passport, gtin);

    assert_eq!(shell.id_short, "DigitalProductPassport");
    assert!(shell.asset_information.global_asset_id.contains(gtin));

    // Five core submodels, plus one per-sector submodel when sector_data is set
    for submodel in &submodels {
        println!(
            "{} ({} elements)",
            submodel.id_short,
            submodel.submodel_elements.len()
        );
    }
}
```

A runnable version is in [`examples/passport_to_aas.rs`](examples/passport_to_aas.rs).

## Semantic identifiers

Every `semanticId` this crate emits is either in the `urn:odal-node:` namespace —
our own concept, honestly named — or carries a provenance record in
[`semantic-ids-allowlist.json`](semantic-ids-allowlist.json) naming who verified
it against the authority's own published source, and when.

That rule is **enforced by a test**, not by a comment. Six identifiers claiming
IDTA and ECLASS authority once sat here behind comments asking someone to check
them; a comment cannot fail a build. An allowlist entry missing `verifiedOn` or
`verifiedBy` is refused.

**As of 2026-07-31 this crate emits no third-party identifiers at all.** Seven
were tracked; six were malformed and one was withdrawn because its *semantic*
correspondence to our submodel had never been checked. Each is recorded in that
file under `tracked`, with the correct identifier where it is known, so
restoring one starts from research rather than from a search engine — and a
test asserts nothing in that record is permitted.

That makes the interop claim narrower and true: this crate produces AAS-shaped
output carrying **our own** semantics. It is not IDTA-aligned, and saying so
would be the defect the gate exists to prevent.

Presenting an own-coined concept under a standards-body identifier tells an
integrator's toolchain that our field is semantically identical to a published
concept. If it is not, that is a false claim made in the format most likely to
be consumed automatically.

## Relationship to other crates

| Crate | Role |
|---|---|
| `dpp-domain` | Provides `Passport`, `SectorData` and the sector catalog — required by this crate |
| `dpp-digital-link` | Supplies the GTIN that becomes `globalAssetId`; no dependency in either direction |

## Minimum Rust version

1.96 (MSRV is enforced in CI)

## License

Apache-2.0 — see [LICENSE](../../LICENSE)
