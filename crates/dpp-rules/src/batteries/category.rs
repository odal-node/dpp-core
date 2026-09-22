//! The Art. 1(3) battery categories, as wire strings.
//!
//! 🚨 **One home for a closed enumeration that had three.** The five values are
//! written in the battery JSON schema's `enum`, in `dpp_domain`'s `BatteryType`
//! serde tags, and — until this module — would have been written a third time
//! in the battery plugin to validate them. Two of those three cannot see each
//! other: the plugins are excluded from the workspace and reach `dpp-rules`,
//! never `dpp-domain`.
//!
//! A drifting copy here does not fail loudly. The plugin would accept a
//! category the domain refuses, or refuse one the schema allows, and which of
//! those happens depends on which copy moved.
//!
//! # Why the set is closed
//!
//! Art. 1(3) enumerates exactly five categories, and its second subparagraph
//! gives a tie-break — *"the category to which the strictest requirements
//! apply"* — that only functions over a closed set. An unrecognised value is a
//! reason to reject the record, not to absorb it: the category selects which
//! obligations apply at all, so guessing it wrong computes a determination
//! against the wrong instrument.

/// The five Art. 1(3) categories, in the schema's own order.
///
/// These are the **wire** strings — `"starting-lighting-ignition"`, not
/// `"sli"` — because this crate is `no_std` and is read by the Wasm plugins,
/// which see JSON and never a Rust enum.
///
/// 🚨 `"sli"` is **not** a member. [`crate::batteries::passport_content`]
/// accepts it as an alias when answering questions about a category, which is
/// tolerance on the read path; it is not a value a passport may carry.
pub const BATTERY_TYPES: [&str; 5] = [
    "portable",
    "industrial",
    "ev",
    "lmt",
    "starting-lighting-ignition",
];
