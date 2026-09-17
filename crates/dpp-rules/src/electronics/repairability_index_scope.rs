//! Which products the repairability index reaches — Reg. (EU) 2023/1669 **Art. 1**.
//!
//! ✅ COMPLIANCE-PIN: EU 2023/1669, Art. 1 (OJ L 214, 31.8.2023, p. 12). Read
//! from the Official Journal text.
//!
//! ## Two regulations, two scopes, and the wider one is the one `DeviceType` follows
//!
//! Art. 1 of **2023/1669** — the energy-labelling regulation that enacts the
//! repairability index — establishes requirements for *"smartphones and slate
//! tablets"*. Art. 1(1) of **2023/1670**, its ecodesign twin, reaches
//! *"smartphones, other mobile phones, cordless phones and slate tablets"*.
//!
//! `DeviceType`'s four values come from the wider one, which is correct for what
//! that type is for and wrong as an input to this index. A cordless phone is not
//! a "mobile phone" under 2023/1670 Art. 2(1) at all, and neither it nor a
//! non-smart mobile phone is in 2023/1669's scope.
//!
//! ## The two exclusions, which are the reason this is not a two-line match
//!
//! Art. 1 continues, and both regulations carry the carve-out in identical words:
//!
//! > This Regulation does not apply to the following products:
//! > (a) mobile phones and tablets with a flexible main display which the user
//! > can unroll and roll up partly or fully;
//! > (b) smartphones for high security communication.
//!
//! A rollable-display phone and a high-security smartphone are both smartphones.
//! Nothing about the device type distinguishes them, so the exclusion has to be
//! **declared** — which is why this function takes it as a second argument rather
//! than deriving it.
//!
//! ## Why an undeclared exclusion means *not excluded*
//!
//! The carve-out is what removes the obligation, so silence cannot grant it: an
//! operator who says nothing has not claimed an exclusion, and reading the
//! absence as one would exempt a product on the strength of a missing field.
//!
//! That is the same fail-closed direction as
//! [`PassportScope::CapacityUnknown`](crate::batteries::passport_scope::PassportScope::CapacityUnknown)
//! and the opposite arithmetic. There the *obligation* turns on a number, so an
//! unstated number cannot exempt; here the *exemption* is what is stated, so an
//! unstated exemption does not exist. Both refuse to exempt on a missing field,
//! which is the property worth holding onto rather than the rule that produces it.
//!
//! ## What this does not answer
//!
//! Whether a passport should carry the index or its inputs at all. **The word
//! "passport" does not occur in either regulation** — the index belongs on the
//! label and in the product information sheet. That is a product question, and no
//! amount of further reading converts it into a legal one.

/// Whether Reg. (EU) 2023/1669's repairability index reaches a product.
///
/// Three outcomes rather than a `bool`. "This device type is outside the act"
/// and "this unit is carved out of it" are different sentences to put in front of
/// an operator, and they have different remedies — the first is never fixable,
/// the second is a declaration the operator made. Same reasoning as
/// [`PassportScope`](crate::batteries::passport_scope::PassportScope).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RepairabilityIndexScope {
    /// Art. 1 names this device type and no exclusion is declared.
    Covered,
    /// Art. 1 does not name this device type — a cordless phone, or a mobile
    /// phone that is not a smartphone. In scope of 2023/1670, not of this act.
    NotCovered,
    /// Art. 1(a) — a flexible main display the user can unroll and roll up.
    ExcludedRollableDisplay,
    /// Art. 1(b) — a smartphone for high security communication.
    ExcludedHighSecurity,
}

impl RepairabilityIndexScope {
    /// Whether an index is owed on the strength of this answer alone.
    ///
    /// `true` only for [`Covered`](Self::Covered). A caller that needs to tell
    /// *why not* apart must match the variant, which is why this is not an
    /// `Option<bool>`.
    #[must_use]
    pub fn is_covered(self) -> bool {
        matches!(self, Self::Covered)
    }

    /// Whether the answer rests on an exclusion the operator declared.
    ///
    /// Exists so a caller can distinguish "the act never reached this" from
    /// "the act reaches this type and the operator says this unit is carved
    /// out" — the second is a claim that can be wrong, and the first cannot.
    #[must_use]
    pub fn is_declared_exclusion(self) -> bool {
        matches!(
            self,
            Self::ExcludedRollableDisplay | Self::ExcludedHighSecurity
        )
    }
}

/// Whether `device_type` owes a repairability index under Reg. (EU) 2023/1669.
///
/// Both arguments are **wire** names — `"smartphone"`, `"tablet"`,
/// `"other-mobile-phone"`, `"cordless-phone"` and `"rollable-display"`,
/// `"high-security-communication"` — because this crate is `no_std` and
/// zero-dependency and is read by the Wasm product group plugins, which see JSON
/// and never the Rust enums. Matching is case-insensitive and trims surrounding
/// whitespace, as in [`crate::batteries::passport_scope`].
///
/// `exclusion` is `None` where the operator declared none, which means **not
/// excluded** — see the module documentation for why silence cannot grant a
/// carve-out. An unrecognised exclusion string is likewise not an exclusion: a
/// value this build does not know is not a value it may act on.
///
/// The device-type check runs **first**. A cordless phone declaring a rollable
/// display answers [`NotCovered`](RepairabilityIndexScope::NotCovered), because
/// the act never reached it and an exclusion from an act that does not apply is
/// not the reason to give an operator.
#[must_use]
pub fn repairability_index_scope(
    device_type: &str,
    exclusion: Option<&str>,
) -> RepairabilityIndexScope {
    let t = device_type.trim();
    let named = t.eq_ignore_ascii_case("smartphone") || t.eq_ignore_ascii_case("tablet");
    if !named {
        return RepairabilityIndexScope::NotCovered;
    }

    match exclusion.map(str::trim) {
        Some(e) if e.eq_ignore_ascii_case("rollable-display") => {
            RepairabilityIndexScope::ExcludedRollableDisplay
        }
        Some(e) if e.eq_ignore_ascii_case("high-security-communication") => {
            RepairabilityIndexScope::ExcludedHighSecurity
        }
        _ => RepairabilityIndexScope::Covered,
    }
}
