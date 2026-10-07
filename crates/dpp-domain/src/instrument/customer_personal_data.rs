//! [`CustomerPersonalData`] — what an act says about storing its customers'
//! personal data in its passport.

use serde::{Deserialize, Serialize};

use crate::personal_data::LawfulBasis;

/// The condition an act sets before personal data relating to customers may be
/// stored in its passport.
///
/// Recorded per act because only some acts set one. ESPR Art. 10(1)(e), toys
/// Art. 20(10) and detergents Art. 22(h) do. The Batteries Regulation does not,
/// so its passport answers to GDPR alone and its manifest carries no entry.
/// Absence is therefore "this act adds no condition", never "personal data may
/// be stored freely": GDPR applies to every passport whatever its manifest says.
///
/// A delegated or implementing act inherits its framework's entry through
/// [`Instrument::parent`](crate::instrument::Instrument::parent), because the
/// framework's passport requirements are what its delegated acts implement. See
/// [`InstrumentCatalog::customer_personal_data_for`](crate::instrument::InstrumentCatalog::customer_personal_data_for).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomerPersonalData {
    /// The condition.
    pub storage: PersonalDataStorage,
    /// The provision that sets it, e.g. `"Regulation (EU) 2024/1781 Art.
    /// 10(1)(e)"`, for a reader to check against the act's text.
    pub provision: String,
}

/// What an act requires before customer personal data may be stored in its
/// passport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum PersonalDataStorage {
    /// Only with the explicit consent of the person concerned, in compliance
    /// with GDPR Art. 6 — which is to say on the basis of its point (a), and no
    /// other.
    ExplicitConsentOnly,
}

impl CustomerPersonalData {
    /// Whether personal data held on `basis` meets this act's condition.
    #[must_use]
    pub fn admits(&self, basis: LawfulBasis) -> bool {
        match self.storage {
            PersonalDataStorage::ExplicitConsentOnly => basis == LawfulBasis::Consent,
        }
    }
}
