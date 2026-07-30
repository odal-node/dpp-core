# dpp-digital-link

[![crates.io](https://img.shields.io/crates/v/dpp-digital-link.svg)](https://crates.io/crates/dpp-digital-link)
[![docs.rs](https://img.shields.io/docsrs/dpp-digital-link)](https://docs.rs/dpp-digital-link)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](../../LICENSE)

GS1 Digital Link URL parsing and building, and GS1 link-type content negotiation,
for the [Odal Node](https://odal-node.io) Digital Product Passport system.

Pure Rust, no I/O, no network calls. `wasm32-unknown-unknown` safe.

> **Scope note.** This crate previously also carried AAS submodel mapping and a
> JSON-LD context. Those are now [`dpp-aas`](../dpp-aas) and
> [`dpp-vc`](../dpp-vc) respectively — a consumer that wants a GS1 parser no
> longer compiles an AAS mapper to get one.

## When to use this crate

- You need to parse or build GS1 Digital Link URLs (GTIN, serial, batch).
- You are resolving DPP links and need to negotiate by link type, media type or
  requesting audience.
- You need GS1 check-digit validation, or a QR-ready canonical URL for a
  passport.

## Example

```rust
use dpp_digital_link::{
    DigitalLink, DppMediaType, Gs1LinkType, LinkDescriptor, ResolutionRequest, negotiate,
};
use dpp_domain::Disclosure;

// Parse a GS1 Digital Link URL
let link = DigitalLink::parse("https://id.gs1.org/01/09521234543213/21/ABC123").unwrap();
assert_eq!(link.gtin.as_str(), "09521234543213");
assert_eq!(link.serial.as_deref(), Some("ABC123"));

// Negotiate the best link for a JSON consumer
let descriptors = vec![LinkDescriptor {
    link_type: Gs1LinkType::DigitalProductPassport,
    media_type: DppMediaType::Json,
    disclosure: Disclosure::Public,
    href: "https://api.odal-node.io/dpp/09521234543213/data".into(),
    title: Some("DPP JSON".into()),
    language: None,
}];

let request = ResolutionRequest {
    link_type: Some(Gs1LinkType::DigitalProductPassport),
    media_type: Some(DppMediaType::Json),
    audience: None,
};

let best = negotiate(&descriptors, &request);
assert!(best.is_some());
```

A runnable version is in [`examples/parse_and_negotiate.rs`](examples/parse_and_negotiate.rs).

## Scope boundary

Syntax is not semantics. This crate proves a link is well-formed GS1; it does
not prove the GTIN is allocated to you, nor that a resolver will find anything
at the other end. Nothing here supports the phrase "GS1-certified".

## Relationship to other crates

| Crate | Role |
|---|---|
| `dpp-domain` | Provides `Audience` and `Disclosure` — required by this crate |
| `dpp-aas` | The AAS projection that used to live here; no dependency in either direction |

## Minimum Rust version

1.96 (MSRV is enforced in CI)

## License

Apache-2.0 — see [LICENSE](../../LICENSE)
