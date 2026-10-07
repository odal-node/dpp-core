//! Port trait for **personal data held outside a passport**, in records that
//! can be erased.
//!
//! [`crate::personal_data`] explains why such data is never inside a passport:
//! consent can be withdrawn at any time (GDPR Art. 7(3)), withdrawal obliges the
//! controller to erase (Art. 17(1)(b)), and a published passport cannot be
//! erased from. A passport carries only a statement naming a record and the
//! basis it is held on. This port holds the record.
//!
//! # The contract
//!
//! - **A record can be erased at any time, and erasing it touches nothing
//!   else.** Not the passport, which is signed and frozen. Not the statement in
//!   it, which keeps naming the record. Not any other record.
//! - **Erasure leaves a tombstone, and the tombstone holds no personal data.**
//!   It keeps the identifier, the passport and field the record belonged to, and
//!   when it was erased, so a controller can show that it erased (GDPR Art.
//!   5(2)) and a reader following the passport's statement learns that the record
//!   is gone rather than that it never existed.
//! - **An identifier is minted here and relates to no one.** It goes into a
//!   signed passport that outlives the record, so it must carry nothing derived
//!   from the data or the person, and it is never reused for another record.
//! - **Nothing here is part of the passport's other copies.** The back-up copy of
//!   ESPR Art. 10(4) and the archive of past versions hold the passport, and so
//!   only the statement. A record lodged with them could not be erased with
//!   this one.
//! - **No passport view includes a record.** Serving one to a reader is a
//!   disclosure that needs a basis of its own, which only the controller can
//!   establish for each purpose; GDPR Art. 25(2) makes withholding it the
//!   default.
//!
//! The port does not read a record's content, and it does not know the basis a
//! record is held on. The basis is the passport's statement, signed by the
//! controller.

mod held;
mod port;
mod receipt;
mod record;
#[cfg(any(test, feature = "test-utils"))]
pub mod stub;
#[cfg(test)]
mod tests;

pub use held::HeldRecord;
pub use port::PersonalDataPort;
pub use receipt::ErasureReceipt;
pub use record::PersonalDataRecord;
