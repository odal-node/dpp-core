//! Unsold consumer products — the ESPR Art. 25 destruction ban (Annex VII) and
//! the Art. 24 disclosure duty (Impl. Reg. (EU) 2026/2).
//!
//! The two have **different scopes** and are kept in separate modules for that
//! reason: the ban reaches apparel and footwear, the disclosure reaches consumer
//! products generally. See [`disclosure`] for what that difference costs if it
//! is collapsed.
pub mod annex_vii;
pub mod disclosure;
