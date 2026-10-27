# dpp-registry

[![crates.io](https://img.shields.io/crates/v/dpp-registry.svg)](https://crates.io/crates/dpp-registry)
[![docs.rs](https://img.shields.io/docsrs/dpp-registry)](https://docs.rs/dpp-registry)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](../../LICENSE)

Interface types for the EU Digital Product Passport registry (ESPR Art. 13,
Implementing Regulation (EU) 2026/1778): registration payloads, the four persistent
identifiers, registration granularity, endpoints, responses, errors and transfer
notifications. Part of [dpp-core](https://github.com/odal-node/dpp-core) by
[Odal Node](https://odal-node.io).

No I/O, no HTTP client, no async runtime. Builds for `wasm32-unknown-unknown`. The
application provides the HTTP adapter that makes the calls, typically by implementing
`RegistrySyncPort` from `dpp-domain`.

## Status of the shapes

What the Official Journal fixes has been reconciled against it: registration
granularity and identifier linking, commodity codes, the operator-identifier scheme,
asynchronous validation. What it does not fix is still open, because the registry's
API specification is unpublished:

- **Authentication** is the least known part. A token in front of the API fits the act
  (IR 2026/1778 Art. 2(3) defines an authentication token), but how one is obtained is
  unpublished, so `RegistryEndpoint::token_endpoint` and any exchange against it are
  this crate's own construction. Do not treat them as an implementation target. A token
  also proves nothing about the verified status of Art. 4, which needs a qualified
  electronic seal or a qualified electronic attestation of attributes.
- **Endpoint paths** are a mix: some are observed in the registry's own published web
  client and some are invented. The `endpoint` module documents which is which for each
  one. An observation is better evidence than an invention, and still not a
  specification.

The item-by-item account, settled and open, is the EU Registry Readiness section of
[`docs/regulatory/COMPLIANCE.md`](../../docs/regulatory/COMPLIANCE.md).

## When to use this crate

- You implement `RegistrySyncPort` and need the request and response types.
- You model passport registration and transfer-notification payloads for the EU
  registry.

## Example

Identifiers validate structurally before any network call, so a bad GTIN check digit
is caught here rather than by the EU endpoint:

```rust
use dpp_registry::ProductIdentifier;

let product = ProductIdentifier {
    scheme: "gtin".into(),
    value: "09506000134352".into(),
    label: Some("EcoCell 18650".into()),
};
assert!(product.validate().is_ok());

// Same GTIN with a corrupted check digit
let corrupted = ProductIdentifier {
    scheme: "gtin".into(),
    value: "09506000134353".into(),
    label: None,
};
assert!(corrupted.validate().is_err());
```

`RegistrationPayload` carries the four Annex III identifier classes (product, item,
facility and operator), and `RegistrationPayload::validate` checks them together
before a round-trip. A runnable version is in
[`examples/registry_identifiers.rs`](examples/registry_identifiers.rs).

## Relationship to other crates

| Crate | Role |
|---|---|
| `dpp-domain` | Provides `Passport`, `PassportId`, `DppError` and the `ProductIdentifier` this crate maps from; required |
| `dpp-rules` | Shared identifier syntax (GTIN check digits); required |

## Minimum Rust version

1.96, enforced in CI.

## Licence

Apache-2.0. See [LICENSE](../../LICENSE).
