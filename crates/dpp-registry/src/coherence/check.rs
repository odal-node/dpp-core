//! [`RegistrationPayload::check_coherence`] — the registration data against the passport.

use dpp_domain::Passport;

use crate::error::RegistryValidationError;
use crate::granularity::Granularity;
use crate::identifiers::ProductIdentifier;
use crate::payload::RegistrationPayload;

impl RegistrationPayload {
    /// Check that this registration says nothing the passport it registers
    /// contradicts, and links what the passport shows exists.
    ///
    /// ✅ COMPLIANCE-PIN: IR (EU) 2026/1778 **Art. 8(7)** — upon submission the
    /// Commission reads the passport content and confirms *"(b) where relevant,
    /// coherence of the mandatory data to be uploaded in the registry against
    /// the value of the data provided in the digital product passport"* and
    /// *"(c) the conformity of the digital product passport with the
    /// granularity level"*. **Art. 8(4)** links the batch and model identifiers
    /// to an item-level passport *"where batch and model design exist for the
    /// product"*, and **Art. 8(5)** the model identifier to a batch-level one
    /// *"where model design exists for the product"*.
    ///
    /// # A value the passport does not state is not compared
    ///
    /// The registry reads what the passport discloses to it, and a value it
    /// cannot see is not one it can find a contradiction in. So a field is
    /// compared only where the passport states it: the batch identifier, which
    /// a passport's public view withholds unless its product group opens it, is
    /// then simply not checked. What the passport states and the registration
    /// leaves out is reported only for the links Art. 8(4) and 8(5) require.
    ///
    /// # Errors
    ///
    /// [`RegistryValidationError::Incoherent`] for the first field that
    /// disagrees, or for a required link left out.
    /// [`RegistryValidationError::UnmappedIdentifierScheme`] if the passport's
    /// identifier is under a scheme this crate has no registry value for.
    pub fn check_coherence(&self, passport: &Passport) -> Result<(), RegistryValidationError> {
        let level = self.level.granularity;

        // The carrier the registration is filed under is the passport's own.
        compare(
            "digitalLinkUrl",
            Some(&self.digital_link_url),
            passport.qr_code_url.as_deref(),
        )?;

        // The identifier, in the registry's vocabulary on both sides.
        if let Some(identifier) = passport
            .product_group_data
            .as_ref()
            .and_then(|data| data.product_identifier())
        {
            let stated = ProductIdentifier::try_from(identifier)?;
            compare(
                "productId",
                Some(&format!(
                    "{}:{}",
                    self.product_id.scheme, self.product_id.value
                )),
                Some(&format!("{}:{}", stated.scheme, stated.value)),
            )?;
        }

        compare(
            "productGroup",
            Some(&self.product_group),
            Some(passport.product_group.wire_str()),
        )?;

        // Art. 8(7)(c): the level the passport states is the level registered.
        // The passport's level is a different type from this crate's, so the
        // two meet on the wire value both serialise to.
        if let Some(stated) = passport
            .granularity
            .and_then(|g| serde_json::to_value(g).ok())
            .and_then(|v| v.as_str().map(ToOwned::to_owned))
        {
            compare("level", Some(level.wire_str()), Some(&stated))?;
        }

        let model = passport
            .product_group_data
            .as_ref()
            .and_then(|data| data.model_identifier());
        compare("level.modelId", self.level.model_id.as_deref(), model)?;

        let batch = passport
            .batch_id
            .as_deref()
            .filter(|b| !b.trim().is_empty());
        compare("level.batchId", self.level.batch_id.as_deref(), batch)?;

        // Art. 8(4)–(5): a link the passport shows exists must be made.
        let links_model = matches!(level, Granularity::Batch | Granularity::Item);
        if links_model {
            require_link("level.modelId", self.level.model_id.as_deref(), model)?;
        }
        if level == Granularity::Item {
            require_link("level.batchId", self.level.batch_id.as_deref(), batch)?;
        }

        if let Some(code) = &passport.commodity_code {
            compare(
                "commodityCode",
                self.commodity_code.as_deref(),
                Some(code.as_str()),
            )?;
        }

        compare(
            "operatorId",
            Some(&self.operator_id.value),
            passport
                .operator_identifier
                .as_deref()
                .filter(|v| !v.trim().is_empty()),
        )?;

        compare(
            "facilityId",
            Some(&self.facility_id.value),
            passport
                .facility
                .as_ref()
                .map(|f| f.value.as_str())
                .filter(|v| !v.trim().is_empty()),
        )?;

        Ok(())
    }
}

/// Refuse a registered value the passport states differently. Either side
/// being silent is not a contradiction.
fn compare(
    field: &'static str,
    registered: Option<&str>,
    in_passport: Option<&str>,
) -> Result<(), RegistryValidationError> {
    match (registered, in_passport) {
        (Some(registered), Some(stated)) if registered != stated => {
            Err(RegistryValidationError::Incoherent {
                field,
                registered: Some(registered.to_owned()),
                in_passport: stated.to_owned(),
            })
        }
        _ => Ok(()),
    }
}

/// Refuse a link the article requires, left out although the passport states
/// what it would link to.
fn require_link(
    field: &'static str,
    registered: Option<&str>,
    in_passport: Option<&str>,
) -> Result<(), RegistryValidationError> {
    match (registered, in_passport) {
        (None, Some(stated)) => Err(RegistryValidationError::Incoherent {
            field,
            registered: None,
            in_passport: stated.to_owned(),
        }),
        _ => Ok(()),
    }
}
