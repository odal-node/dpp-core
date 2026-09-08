//! [`LifeStatus`] — where a unit sits in its product life, per Annex XIII 4(c).

use serde::{Deserialize, Serialize};

/// The product-life status of an individual battery.
///
/// ✅ COMPLIANCE-PIN: EU 2023/1542, Annex XIII point 4(c)
/// (OJ L 191, 28.7.2023, p. 109) — "information on the status of the battery,
/// defined as 'original', 'repurposed', 're-used', 'remanufactured' or
/// 'waste'".
///
/// # Not the same thing as `PassportStatus`
///
/// [`PassportStatus`](crate::status::PassportStatus) is a
/// **publication** lifecycle: draft, published, suspended, archived, superseded,
/// deactivated. This is a **product-life** status. They are orthogonal, and the
/// clearest case is a repurposed unit, whose passport is perfectly ordinarily
/// `Published` while its life status is `Repurposed`. Before this type existed
/// the change-of-status information the Regulation requires had nowhere to live.
///
/// # The list is closed
///
/// Point 4(c) enumerates the five values, so a sixth would be an invention
/// rather than an extension. In particular there is no "approaching end of
/// life": that phrase appears nowhere in Regulation (EU) 2023/1542, and an
/// earlier draft of the design note carried it in place of `'original'`.
///
/// # Not public
///
/// Point 4 sits under the heading "INFORMATION AND DATA RELATING TO AN
/// INDIVIDUAL BATTERY ACCESSIBLE ONLY TO PERSONS WITH A LEGITIMATE INTEREST",
/// so this field is classified
/// [`Disclosure::Individual`](crate::disclosure::Disclosure::Individual) in
/// [`PASSPORT_FIELD_DISCLOSURE`](crate::disclosure::PASSPORT_FIELD_DISCLOSURE).
/// That classification is load-bearing rather than decorative: the passport
/// policy's `default_disclosure` is `Public`, so an envelope field nobody
/// classifies is served to anonymous readers.
///
/// # Four of the five are set at create; one is not
///
/// Art. 77(7) makes each of the four second-life operations produce a **new**
/// passport, so a second-life unit is born already knowing its status.
/// `Waste` is the exception, and the difference is structural: Art. 77(7)'s
/// second subparagraph moves responsibility on a battery becoming waste and
/// mandates **no new passport**, while point 4(a) expects values reported "when
/// the battery is placed on the market and when it is subject to changes in its
/// status". A create-time-only field could therefore never reach `Waste` — one
/// of the five values the law enumerates.
///
/// The transition is nonetheless not a free patch: `lifeStatus` is in
/// [`PROTECTED_PATCH_FIELDS`](crate::ports::passport_repo::PROTECTED_PATCH_FIELDS),
/// so moving a published record to `Waste` is a new passport **version** via
/// `supersedes_id`, which keeps the signature honest and leaves an audit trail.
/// A dedicated port method would have had to re-sign the served body, which is a
/// version bump wearing a disguise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum LifeStatus {
    /// The unit as originally placed on the market — no second-life operation
    /// has been performed on it.
    #[serde(rename = "original")]
    Original,
    /// Art. 3(31) — used for a purpose other than the one it was designed for,
    /// the input not being a waste battery.
    #[serde(rename = "repurposed")]
    Repurposed,
    /// Art. 3(29) — prepared for re-use, as defined in Art. 3, point (16), of
    /// Directive 2008/98/EC.
    ///
    /// The wire form is `re-used`, hyphenated, because that is the string
    /// Annex XIII point 4(c) enumerates. See the type's note on wire forms.
    #[serde(rename = "re-used")]
    Reused,
    /// Art. 3(32) — disassembly and evaluation of all cells and modules with
    /// enough of them reused to restore at least 90 % of the original rated
    /// capacity, for the same purpose as originally designed.
    #[serde(rename = "remanufactured")]
    Remanufactured,
    /// The battery has become waste.
    ///
    /// The one value that is a transition *on a record that continues*, rather
    /// than a property a new passport is born with — see the type documentation.
    #[serde(rename = "waste")]
    Waste,
}

impl LifeStatus {
    /// Every status this build models, for exhaustive iteration.
    ///
    /// `LifeStatus` is `#[non_exhaustive]`, so a consumer outside this crate
    /// cannot enumerate it, and one publishing an API description has to. See
    /// [`TransferReason::ALL`](crate::transfer::TransferReason::ALL) for the
    /// same contract.
    ///
    /// The list is closed by the annex: point 4(c) names five values and no
    /// delegated act may add a sixth without amending it.
    pub const ALL: &'static [Self] = &[
        Self::Original,
        Self::Repurposed,
        Self::Reused,
        Self::Remanufactured,
        Self::Waste,
    ];

    /// The stable wire form, for payloads that carry the status as a string.
    ///
    /// Spelled out rather than derived from `Serialize` so that renaming a
    /// variant cannot silently change what a registry receives — the same
    /// contract as
    /// [`SecondLifeOperation::wire_str`](crate::passport::SecondLifeOperation::wire_str).
    ///
    /// # These are the Official Journal's own strings
    ///
    /// Every other wire vocabulary in this crate is camelCase, and this one is
    /// not, deliberately. Annex XIII point 4(c) does not name concepts for us to
    /// spell as we like — it *enumerates the literal values* the status is
    /// "defined as". `re-used` keeps its hyphen for that reason: writing
    /// `reused` would be inventing a value the instrument does not contain, and
    /// the point of pinning a citation is that a reader can check it against the
    /// primary source and find the same string.
    #[must_use]
    pub fn wire_str(&self) -> &'static str {
        match self {
            Self::Original => "original",
            Self::Repurposed => "repurposed",
            Self::Reused => "re-used",
            Self::Remanufactured => "remanufactured",
            Self::Waste => "waste",
        }
    }
}
