# dpp-evidence

[![crates.io](https://img.shields.io/crates/v/dpp-evidence.svg)](https://crates.io/crates/dpp-evidence)
[![docs.rs](https://img.shields.io/docsrs/dpp-evidence)](https://docs.rs/dpp-evidence)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](../../LICENSE)

Evidence dossier format and offline verification for the [Odal Node](https://odal-node.io)
Digital Product Passport system: a self-contained, signed export of a passport's full proof
chain — JWS signatures, hash-chained audit trail, transfer-chain signatures — verifiable by
anyone with zero trust in the issuing node.

## When to use this crate

- You are assembling an evidence dossier for a passport (an engine/platform adapter).
- You are building a verifier — CLI, wasm/browser, or otherwise — that checks a dossier
  fully offline.
- You need the audit-trail hash-chain type (`AuditEntry`) or its chain-verification
  algorithm independent of any particular storage backend.

## Example

```rust
use dpp_evidence::{VerifyMode, verify_dossier_json};

let bytes = std::fs::read("dossier.json").unwrap();
let report = verify_dossier_json(&bytes, VerifyMode::Embedded).unwrap();

if report.all_verified() {
    println!("VERIFIED — {}", report.trust_anchor_note);
} else {
    for check in &report.checks {
        println!("{}: {:?}", check.name, check.status);
    }
}
```

## Relationship to other crates

| Crate | Role |
|---|---|
| `dpp-domain` | Provides `TransferChain`/`TransferRecord` and passport identifiers — required by this crate |

## Minimum Rust version

1.96 (MSRV is enforced in CI)

## License

Apache-2.0 — see [LICENSE](../../LICENSE)
