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
    ///
    /// **A lost answer does not lose the record.** A retry stores a second one,
    /// so a caller whose `store` failed after the record was kept finds the
    /// first through [`records_for`](Self::records_for) and erases or adopts
    /// it, rather than leaving personal data held that nothing refers to.
    async fn store(
        &self,
        passport_id: PassportId,
        field: &str,
        content: &serde_json::Value,
    ) -> Result<PersonalDataRecordId, DppError>;

    /// What is held under `record`: the record itself, or its tombstone if it
    /// was erased. `None` when this port never minted that identifier.
    async fn fetch(&self, record: &PersonalDataRecordId) -> Result<Option<HeldRecord>, DppError>;

    /// Every record held for a passport, present or erased, in the order they
    /// were stored. Empty when none was.
    ///
    /// This is how a controller answers for everything it holds about one
    /// passport, and how it finds a record whose identifier it lost: one
    /// stored by a `store` whose answer never arrived, or by a caller that
    /// failed before writing the statement naming it. A record that no
    /// statement names is personal data held for no purpose.
    async fn records_for(&self, passport_id: PassportId) -> Result<Vec<HeldRecord>, DppError>;

    /// Erase a record's data, keeping its tombstone.
    ///
    /// **A retry is safe.** Erasing a record already erased returns the receipt
    /// it was given the first time, so a caller that lost the answer can ask
    /// again. An identifier this port never minted is refused with
    /// [`DppError::NotFound`], because there is nothing to have erased.
    async fn erase(&self, record: &PersonalDataRecordId) -> Result<ErasureReceipt, DppError>;
}
