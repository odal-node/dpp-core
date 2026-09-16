//! [`IndexScopeExclusion`] — the two product classes Art. 1 of Reg. (EU)
//! 2023/1669 carves out of the repairability index.
//!
//! ✅ COMPLIANCE-PIN: EU 2023/1669, Art. 1 (OJ L 214, 31.8.2023, p. 12). Read
//! from the Official Journal text.
//!
//! Both exclusions describe products that are otherwise an ordinary smartphone
//! or slate tablet. Nothing else in the record distinguishes them, so the
//! exclusion has to be declared rather than derived — which is the whole reason
//! this type exists.

use serde::{Deserialize, Serialize};

/// A product class Art. 1 of Reg. (EU) 2023/1669 carves out of its own scope.
///
/// Both values are `DeviceType::Smartphone` or `DeviceType::Tablet` as far as
/// every other field is concerned, so without this the index would be claimed
/// over two product classes the Regulation expressly disclaims.
///
/// **Absent means not excluded**, never "unknown". The carve-out is what removes
/// the obligation, so an operator who declares nothing has claimed nothing, and
/// reading silence as an exclusion would exempt a product on a missing field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum IndexScopeExclusion {
    /// Art. 1(a) — *"a flexible main display which the user can unroll and roll
    /// up partly or fully"*. Reaches tablets as well as phones.
    RollableDisplay,
    /// Art. 1(b) — *"smartphones for high security communication"*.
    HighSecurityCommunication,
}

impl IndexScopeExclusion {
    /// Every exclusion this build models, for exhaustive iteration.
    ///
    /// `IndexScopeExclusion` is `#[non_exhaustive]`, so a consumer publishing an
    /// API description cannot enumerate it and would hand-write the list
    /// downstream — where it keeps compiling after a variant is added here and
    /// ships the new value undocumented. Same contract as
    /// [`PassportStatus::ALL`](crate::status::PassportStatus::ALL).
    pub const ALL: &'static [Self] = &[Self::RollableDisplay, Self::HighSecurityCommunication];

    /// The serde wire tag, e.g. `"rollable-display"`.
    #[must_use]
    pub const fn wire_str(&self) -> &'static str {
        match self {
            Self::RollableDisplay => "rollable-display",
            Self::HighSecurityCommunication => "high-security-communication",
        }
    }
}
