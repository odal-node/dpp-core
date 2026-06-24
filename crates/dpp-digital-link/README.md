# dpp-digital-link

GS1 Digital Link URL parsing, link-type content negotiation, and
[Asset Administration Shell](https://industrialdigitaltwin.org/en/content-hub/aasspecifications)
(AAS) submodel mapping for the [Odal Node](https://odal-node.io) Digital Product
Passport system.

Pure Rust, no I/O, no network calls.

## When to use this crate

- You need to parse or build GS1 Digital Link URLs (GTIN, serial, batch).
- You are resolving DPP links and need to negotiate content by link type, media type,
  or access tier.
- You want to map a DPP `serde_json::Value` to an AAS submodel for Industry 4.0
  interoperability.

## Example

```rust
use dpp_digital_link::{DigitalLink, AccessTier, negotiate, Gs1LinkType};

// Parse a GS1 Digital Link URL
let link = DigitalLink::parse(
    "https://id.gs1.org/01/09521234543213/21/ABC123",
).unwrap();
println!("GTIN: {}", link.gtin);
println!("Serial: {}", link.serial.unwrap());

// Negotiate the best link for a professional-tier consumer
let descriptors = vec![/* LinkDescriptor items */];
let best = negotiate(&descriptors, None, None, AccessTier::Professional);
```

## Relationship to other crates

| Crate | Role |
|---|---|
| `dpp-domain` | Provides `AccessTier` — required by this crate |

## Minimum Rust version

1.96 (MSRV is enforced in CI)

## License

Apache-2.0 — see [LICENSE](../../LICENSE)
