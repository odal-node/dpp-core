//! [`ManufacturerInfo`] embedded in the passport.

use serde::{Deserialize, Serialize};

/// Manufacturer information embedded in the passport.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManufacturerInfo {
    pub name: String,
    pub address: String,
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
