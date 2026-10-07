//! [`PersonalDataPort`] — the contract a holder of personal data records
//! implements.

use async_trait::async_trait;

use super::held::HeldRecord;
use super::receipt::ErasureReceipt;
use crate::error::DppError;
use crate::passport::PassportId;
use crate::personal_data::PersonalDataRecordId;

/// Port trait for holding personal data outside a passport, erasably.
///
/// The module docs state the contract; the method docs state what each call
/// owes.
#[async_trait]
pub trait PersonalDataPort: Send + Sync {
    /// Hold `content`, personal data related to the marked `field` of a
    /// passport, and return the identifier its statement will name.
    ///
    /// Every call mints a new identifier, unrelated to the content, the person
    /// or any earlier record. Storing does not check that `field` is marked:
    /// that is the passport's check, made when the statement naming the record
    /// is written.
    async fn store(
        &self,
        passport_id: PassportId,
        field: &str,
        content: &serde_json::Value,
    ) -> Result<PersonalDataRecordId, DppError>;

    /// What is held under `record`: the record itself, or its tombstone if it
    /// was erased. `None` when this port never minted that identifier.
    async fn fetch(&self, record: &PersonalDataRecordId) -> Result<Option<HeldRecord>, DppError>;

    /// Erase a record's data, keeping its tombstone.
    ///
    /// **A retry is safe.** Erasing a record already erased returns the receipt
    /// it was given the first time, so a caller that lost the answer can ask
    /// again. An identifier this port never minted is refused with
    /// [`DppError::NotFound`], because there is nothing to have erased.
    async fn erase(&self, record: &PersonalDataRecordId) -> Result<ErasureReceipt, DppError>;
}
