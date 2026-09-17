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
    /// The unique product identifier this record carries, in whichever
    /// EN 18219 clause 5 scheme issued it.
    ///
    /// `None` is a real answer: a disclosure covering many products identifies
    /// no single product the way every other group does.
    ///
    /// This is the method a group implements. [`Self::gtin`] is derived from it
    /// and is not overridden anywhere — asking a payload for its GTIN is asking
    /// a question only one of the three schemes can answer.
    fn product_identifier(&self) -> Option<&crate::identifier::ProductIdentifier>;

    /// The GS1 trade item number, when this record's identifier was issued
    /// under EN 18219 **scheme 1** and not otherwise.
    ///
    /// 🚨 `None` now has two meanings and a caller has to tolerate both: this
    /// group carries no identifier at all, **or** it carries one from scheme 2
    /// or 3, which are self-issuing and have no GTIN. Before the identifier
    /// became a scheme this could only mean the first, so a caller reading
    /// `None` as "not a product" is now wrong — see
    /// [`Self::product_identifier`] for the answer that always exists.
    fn gtin(&self) -> Option<&str> {
        self.product_identifier()?
            .gtin()
            .map(crate::identifier::Gtin::as_str)
    }

    /// The manufacturer's model identifier, where this group's act defines one.
    ///
    /// `None` means the act defines no such concept — not that the value is
    /// missing.
    fn model_identifier(&self) -> Option<&str>;

    /// Substances of very high concern declared on this payload, where this
    /// group's schema carries them.
    ///
    /// `None` means the schema has no such field — the group was never asked
    /// for the declaration. `Some(&[])` means it was asked and answered
    /// "none present", which is a different claim and the one REACH Art. 33
    /// cares about. The SVHC lints run on the second and stay silent on the
    /// first, so collapsing them would make an unasked group look cleared.
    fn svhc_substances(&self) -> Option<&[crate::product_group::SvhcSubstance]>;

    /// The product category this record declares, as the wire string the
    /// catalog lists in `productCategories`.
    ///
    /// # Why the group has to answer this itself
    ///
    /// There is no common field to read. Seven groups carry a category and name
    /// it seven different things — `batteryType`, `productFamily`,
    /// `productType`, `productCategory`, `tyreClass` — because each act names
    /// its own axis and none of them agreed to call it the same thing. Which
    /// field *is* the category is a fact about the act, so it is answered in
    /// the group's own file, exactly like [`Self::model_identifier`].
    ///
    /// # It must be the category axis, not merely a field with few values
    ///
    /// The answer is what a caller compares against a credential's declared
    /// scope, so it has to be the axis that scope is about. A field that
    /// partitions the group some *other* way — how the metal was made, how old
    /// the child is — has a small enumerated domain and looks the part, and
    /// answering with one would let a credential scoped to a category be
    /// satisfied by something that is not a category. Where the act defines no
    /// such axis the answer is `None`.
    ///
    /// The values themselves are constrained by the group's own schema enum,
    /// the same gate every other validated field passes. Note that this is
    /// **wider** than the catalog's `productCategories`, which is a curated
    /// subset rather than the legal domain — a furniture passport may lawfully
    /// be a `bed`, which the schema allows and the catalog does not list.
    ///
    /// `None` is a real answer, the same way it is for the other two: it says
    /// this group has no category a passport can state, not that a value is
    /// missing.
    fn product_category(&self) -> Option<&str>;
}
