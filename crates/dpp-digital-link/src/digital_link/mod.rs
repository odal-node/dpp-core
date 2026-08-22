//! GS1 Digital Link parser, builder, and GTIN utilities.
//!
//! Canonical Odal form: `https://id.odal-node.io/01/{gtin}/21/{serial}`
//!
//! Supports the GS1 Digital Link standard (GS1 DL URI Syntax, v1.2).
//! Application Identifiers (AIs) recognised in the path:
//! - `01`  — GTIN-14 (primary key; GTIN-8/12/13 normalised to 14 by left-padding)
//! - `22`  — Consumer product variant (qualifier; canonical order 1)
//! - `10`  — Batch/lot number (qualifier; canonical order 2)
//! - `21`  — Serial number (qualifier; canonical order 3)
//! - `235` — Third-party controlled serial (qualifier; canonical order 4)
//!
//! Query parameters (`?…`) are split from the path before segmenting so they
//! can never corrupt the value of the last qualifier.
//! AI values are percent-decoded on parse and percent-encoded on build.
//! The resolver base URL preserves any path prefix that precedes the `/01/`
//! segment, so `https://example.com/resolve/01/…` round-trips correctly.
//!
//! ## Module layout
//!
//! - `ai`    — the recognised Application Identifier table for URI paths.
//! - `syntax_dictionary` — GS1's published Syntax Dictionary, parsed. The
//!   authority for every AI's length and whether it is pre-defined length.
//! - `element_string` — [`ElementString`], the AI data a scanner emits.
//! - `error` — [`DigitalLinkError`].
//! - `codec`   — percent-encode/decode and GTIN normalisation (private helpers).
//! - `link`   — [`DigitalLink`] (parse/build).
//! - `gtin`   — [`validate_gtin`].
//! - `qr`     — [`build_qr_url`].

mod ai;
mod codec;
mod element_string;
mod error;
mod gtin;
mod link;
mod qr;
mod syntax_dictionary;
#[cfg(test)]
mod tests;

pub use ai::{AI_TABLE, AiDescriptor, AiRole, ai_descriptor};
pub use element_string::ElementString;
pub use error::DigitalLinkError;
pub use gtin::validate_gtin;
pub use link::DigitalLink;
pub use qr::{build_qr_url, short_serial};
pub use syntax_dictionary::{AiSpec, ai_len_for_prefix, ai_spec, dictionary};
