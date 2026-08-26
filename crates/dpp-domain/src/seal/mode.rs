//! [`SealMode`] — whether a seal is produced locally or by a remote service.

use serde::{Deserialize, Serialize};

/// Which eIDAS sealing model the request should use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SealMode {
    /// Platform holds its own qualified seal; operators use delegated access.
    ///
    /// **The legal basis for this mode is not established by the registry
    /// rules.** Verified against the OJ text of IR 2026/1778: Art. 19(4) permits
    /// a verified economic operator to authorise a third party to perform
    /// *"registration actions in the registry"* on its behalf, provided that
    /// third party follows the verification process in accordance with Art. 5.
    /// That is delegated **registration**, and it says nothing about who may
    /// hold or use a qualified electronic seal.
    ///
    /// Art. 19(5) is likewise about data rather than seals: each verified
    /// economic operator *"shall be responsible for the data it submits to the
    /// Commission as manager of the registry and shall be considered as the
    /// controller of the data it submits"*.
    ///
    /// So delegation of registration is settled and delegation of sealing is
    /// not. Whether one party may hold a qualified seal covering content another
    /// party authored is a question under eIDAS and the applicable delegated
    /// act, not one these articles answer — and the mechanics moving would not
    /// move the responsibility either way.
    ProviderSeal,
    /// Operator holds and manages their own qualified seal.
    OperatorSeal,
}

impl SealMode {
    /// Every mode this build models. Same reasoning as [`SealFormat::ALL`](crate::seal::SealFormat::ALL).
    pub const ALL: &'static [Self] = &[Self::ProviderSeal, Self::OperatorSeal];
}
