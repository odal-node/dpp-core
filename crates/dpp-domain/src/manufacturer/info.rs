//! [`ManufacturerInfo`] embedded in the passport.

use serde::{Deserialize, Serialize};

/// Manufacturer information embedded in the passport.
///
/// # The contact obligation this has to satisfy
///
/// ESPR **Art. 27(6)** names four elements, and it is unusually direct about
/// where they go:
///
/// > manufacturers shall indicate their **name, registered trade name or
/// > registered trade mark, postal address** at which, and **electronic means of
/// > communication** through which, they can be contacted: (a) **on the public
/// > part of the digital product passport**, where applicable […]
///
/// Most of Annex III lists what a delegated act *may* specify for a product
/// group. This is not that: it is a direct duty on the manufacturer for any
/// product a delegated act covers, and the act's only role is the "where
/// applicable" of whether the product has a passport at all. That makes it the
/// strongest data-content claim in the Regulation on this type.
///
/// The same four-element floor appears in ESPR Art. 29(3) (importer),
/// Art. 4(4) of Regulation (EU) 2019/1020, and Art. 16(3) of Regulation (EU)
/// 2023/988. [`ResponsibleOperator`](crate::operator::ResponsibleOperator)
/// already carries it for the operator side; this type reuses its **field
/// names** rather than sharing a type, because the two records differ in what
/// else they carry and in which of their fields are required.
///
/// # What is still not enforceable
///
/// Art. 27(6) closes with *"The address shall indicate a single point where the
/// manufacturer can be contacted."* [`address`](Self::address) is a free string,
/// so nothing here can check that — and a structured address could not either,
/// because "single point" is a property of the content rather than the shape. It
/// is stated so that a reader knows it is unchecked, not to imply it is.
///
/// # Why these fields are public
///
/// `manufacturer` carries no entry in
/// [`PASSPORT_FIELD_DISCLOSURE`](crate::disclosure::PASSPORT_FIELD_DISCLOSURE),
/// so it and its nested keys default to [`Public`](crate::disclosure::Disclosure::Public).
/// For these two that is not an oversight to be tidied up later — Art. 27(6)(a)
/// puts the contact details **on the public part** specifically, so classifying
/// them anywhere else would break the obligation the fields exist to satisfy.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManufacturerInfo {
    /// The manufacturer's legal entity name.
    ///
    /// One of the three alternatives Art. 27(6) offers for identification; the
    /// trading name is [`registered_trade_name`](Self::registered_trade_name)
    /// and is frequently different.
    pub name: String,
    /// Postal address at which the manufacturer can be contacted.
    pub address: String,
    /// Registered trade name or registered trade mark, where it differs from
    /// [`name`](Self::name).
    ///
    /// Art. 27(6) names it separately from the legal name — *"name, registered
    /// trade name or registered trade mark"* — because the two routinely differ,
    /// and the one a reader holding the product sees is the trading one. A
    /// single `name` field doing duty for both cannot say which it holds, so a
    /// consumer matching a product to its manufacturer has nothing reliable to
    /// match on.
    ///
    /// `Option`, because the envelope is additive-only: a document written
    /// before this field existed deserialises as `None`. `None` means **not
    /// stated** — including the common case where the legal name *is* the
    /// trading name and there is nothing separate to record.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registered_trade_name: Option<String>,
    /// Electronic means of communication through which the manufacturer can be
    /// contacted — an email address, or a contact URL.
    ///
    /// **Not** [`did_web_url`](Self::did_web_url), and the distinction is the
    /// reason this field exists. A DID document resolves keys. Nothing in it is
    /// required to be a mailbox or a form, so it is not a channel through which
    /// a person can be contacted, and treating it as one would satisfy
    /// Art. 27(6) on paper while leaving a reader with no way to reach anybody.
    ///
    /// `Option` for the additive reason above; `None` means not stated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub electronic_address: Option<String>,
    /// ISO 3166-1 alpha-2 country of the manufacturer, e.g. `"DE"`.
    ///
    /// # Why this is its own field
    ///
    /// It had none, so it was written into [`address`](Self::address) — five
    /// bulk importers map a `manufacturerCountry` column straight onto it, and
    /// the battery importer additionally accepts a full `manufacturerAddress`
    /// under the same alias, so nothing downstream can tell which of the two a
    /// given passport carries.
    ///
    /// Country is structural rather than descriptive. Whether an operator is
    /// established in the Union is what separates a manufacturer from an
    /// importer or an authorised representative, and that distinction decides
    /// who carries the passport obligation. The registry also identifies
    /// operators by EORI, which is country-prefixed and cannot be cross-checked
    /// against a country buried in free text.
    ///
    /// Every product group's own `countryOfOrigin` is constrained to
    /// `^[A-Z]{2}$` in its JSON Schema. The manufacturer's country was the one
    /// that had no field to constrain.
    ///
    /// `Option`, because the envelope is additive-only: a document written
    /// before this field existed must still deserialise, and it does, as `None`.
    /// `None` means "not stated" and never "stateless" — it is not a claim
    /// about the manufacturer.
    ///
    /// Validated by [`ManufacturerInfo::validate_country`], which is a
    /// **membership** check against the assigned ISO 3166-1 alpha-2 set, not a
    /// two-uppercase-letters shape check: `XX` and `QZ` are refused.
    #[serde(default)]
    pub country: Option<String>,
    /// The manufacturer's `did:web` URL, e.g. `https://acme.example.com/.well-known/did.json`
    pub did_web_url: Option<String>,
}

impl ManufacturerInfo {
    /// `Err` with the offending value when `country` is present and is not an
    /// assigned ISO 3166-1 alpha-2 code. `Ok(())` when it is absent — an
    /// unstated country is not an invalid one.
    ///
    /// Separate from `Passport::validate`'s inline field checks so the rule has
    /// one home: the importers that populate this field validate a row before a
    /// `Passport` exists to validate.
    pub fn validate_country(&self) -> Result<(), &str> {
        match self.country.as_deref() {
            Some(code) if !dpp_rules::country_code_valid(code) => Err(code),
            _ => Ok(()),
        }
    }
}
