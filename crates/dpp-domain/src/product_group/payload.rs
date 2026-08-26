//! [`ProductGroupPayload`] — what every product-group payload can answer about
//! itself.
//!
//! # Why a trait and not a shared struct
//!
//! Eleven payloads declare a `gtin`, nine a `country_of_origin`, six a
//! `recycled_content_pct`. That looks like duplication and mostly is not: the
//! *obligation* differs. Eight acts require country of origin and put it in the
//! schema's `required` block; the battery and electronics acts declare it and do
//! not require it. A shared base has to pick one, and either choice writes a
//! false statement about EU law into a Rust struct.
//!
//! So the sharing here is on **behaviour**, which is safe, rather than on
//! **data**, which would encode a legal claim. Each payload keeps its own fields
//! exactly as its act defines them and answers a common question about them.
//!
//! # No default implementations, deliberately
//!
//! Every method is required. A default returning `None` would let a newly added
//! product group inherit "no model identifier" silently, and a registry would be
//! told that as fact. Requiring the answer makes adding a group a compile error
//! until someone has read the act and written it down — the same property the
//! exhaustive `match` this trait replaced was protecting, but now stated in the
//! group's own file instead of three matches away from it.

/// The questions any product-group payload can answer, whatever its act.
pub trait ProductGroupPayload {
    /// The GS1 trade item number, where this group's act requires one.
    ///
    /// `None` is a real answer: a disclosure covering many products has no single
    /// trade item number to give.
    fn gtin(&self) -> Option<&str>;

    /// The manufacturer's model identifier, where this group's act defines one.
    ///
    /// `None` means the act defines no such concept — not that the value is
    /// missing.
    fn model_identifier(&self) -> Option<&str>;
}
