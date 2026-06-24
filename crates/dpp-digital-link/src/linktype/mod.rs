//! GS1 Digital Link link-type negotiation.
//!
//! When a resolver receives a Digital Link request, the client can specify
//! which representation it wants via:
//! - Query parameter: `?linkType=gs1:epil` (product information page)
//! - HTTP Accept header: `application/json`, `application/ld+json`, etc.
//!
//! The ESPR mandates that DPP data is resolvable through GS1 Digital Link.
//! Different consumers need different representations:
//! - A consumer scanning a QR code wants an HTML product page.
//! - A machine client wants JSON-LD or raw DPP JSON.
//! - A market surveillance authority wants the full signed DPP payload.
//!
//! ## Module layout
//!
//! - `vocabulary`  — [`Gs1LinkType`], the GS1 Web Vocabulary link types.
//! - `media_type`  — [`DppMediaType`] for content negotiation.
//! - `request`     — [`ResolutionRequest`] + HTTP `Accept`-header parsing.
//! - [`negotiate`](negotiate()) — [`LinkDescriptor`] + the negotiation algorithm.

mod media_type;
mod negotiate;
mod request;
#[cfg(test)]
mod tests;
mod vocabulary;

/// Re-export the canonical `AccessTier` from dpp-domain.
pub use dpp_domain::AccessTier;
pub use media_type::DppMediaType;
pub use negotiate::{LinkDescriptor, negotiate};
pub use request::ResolutionRequest;
pub use vocabulary::Gs1LinkType;
