//! `dpp-jsonld` — JSON-LD context for passport payloads.
//!
//! Pure, stateless crate with no I/O or network dependencies. Compiles to both
//! `std` and `wasm32`.
//!
//! Small today. It is a crate rather than a module because the `ld+json` door
//! is expected to grow, and it should not grow inside a GS1 crate — which is
//! how `dpp-digital-link` came to carry four unrelated capabilities behind one
//! name.

pub mod jsonld;

pub use jsonld::*;
