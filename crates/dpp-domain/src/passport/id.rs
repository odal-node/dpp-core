//! [`PassportId`] — the passport's unique identifier.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Newtype wrapper for a passport's unique identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PassportId(pub Uuid);

impl PassportId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    /// The carrier serial a passport gets when its operator attributes none:
    /// the **last ten bytes** of this id as lowercase hex.
    ///
    /// Exactly twenty characters from `[0-9a-f]`, so it fits GS1's AI 21 limit
    /// and sits inside CSET 82 by construction. It is also the last twenty hex
    /// digits of the id's canonical text form, which is what lets storage index
    /// it with an expression over the id rather than a stored copy of it —
    /// `the_default_is_the_tail_of_the_canonical_id` pins that equivalence.
    ///
    /// # Why the last ten bytes, not the first
    ///
    /// Passport IDs are UUIDv7, whose leading six bytes are a big-endian
    /// millisecond timestamp (RFC 9562 §5.7). Taking the *first* ten bytes gave a
    /// serial that sorted in creation order and whose first twelve hex characters
    /// decoded directly to the passport's creation instant — so a code printed on
    /// a physical battery published its creation time to the millisecond, and
    /// any two codes revealed their production order and the rate between them.
    ///
    /// Bytes 6..16 are the `rand_a` and `rand_b` fields: 74 random bits, with only
    /// the 4-bit version and 2-bit variant fixed. No timestamp, no ordering, and
    /// nothing to guess a neighbouring passport's serial from.
    #[must_use]
    pub fn default_carrier_serial(&self) -> String {
        let mut serial = String::with_capacity(20);
        for byte in &self.0.as_bytes()[6..] {
            serial.push_str(&format!("{byte:02x}"));
        }
        serial
    }
}

impl Default for PassportId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for PassportId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
