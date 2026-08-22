//! `dpp-digital-link` — GS1 Digital Link parsing and building, and GS1 link-type
//! negotiation.
//!
//! Pure, stateless crate with no I/O or network dependencies. Compiles to both
//! `std` and `wasm32`.
//!
//! This crate previously also carried AAS mapping and a JSON-LD context behind
//! its name. Both have moved out — the AAS projection to [`dpp-aas`], the
//! JSON-LD context to `dpp-vc` — and neither is reachable from here, which is
//! the point: a consumer that wants a GS1 parser no longer compiles an AAS
//! mapper to get one.
//!
//! [`dpp-aas`]: https://docs.rs/dpp-aas

pub mod digital_link;
pub mod linktype;

pub use digital_link::{
    AI_TABLE, AiDescriptor, AiRole, AiSpec, DigitalLink, DigitalLinkError, ElementString,
    ai_descriptor, ai_len_for_prefix, ai_spec, build_qr_url, dictionary, short_serial,
    validate_gtin,
};
pub use linktype::{
    Audience, DppMediaType, Gs1LinkType, LinkDescriptor, ResolutionRequest, negotiate,
};

/// Compile-checks this crate's README examples.
///
/// A README example is a public claim about the API, and nothing else in the
/// build compiles one. Without this, a README can advertise a function that
/// does not exist — which is exactly what happened before this harness landed.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
