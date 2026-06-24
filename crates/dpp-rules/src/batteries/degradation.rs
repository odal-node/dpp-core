//! Battery degradation rules — SOH, capacity fade, cycle life thresholds.
//!
//! EU Regulation 2023/1542 requires EV and industrial batteries to declare
//! state-of-health (SOH) and capacity fade metrics (Annex IV, Art. 10).
//! The **minimum performance thresholds** for market access — i.e., the values
//! below which a battery would be non-compliant — are to be specified in a
//! **delegated act under Art. 10(6)** that has not yet been adopted (expected
//! 2027–2028). Until that act is published, no compliance determination against
//! SOH or capacity-fade thresholds is legally possible.
//!
//! ## What the schema declares today
//! The battery schema (v2.0.0) carries:
//! - `stateOfHealthPct`       — 0–100, validated by JSON Schema
//! - `expectedLifetimeCycles` — positive integer, validated by JSON Schema
//!
//! There are no cross-field rules between these two fields that can be derived
//! from current regulation text. Both are independently range-checked by JSON
//! Schema and there is no regulatory formula linking them until the delegated
//! act arrives.
//!
//! ## Placeholder note
//! Once the delegated act specifies minimum SOH thresholds (likely differentiated
//! by battery type and application), implement them here and expose them to the
//! battery plugin via `dpp_rules::batteries::degradation`.

// Placeholder — rules to be implemented once Art. 10(6) delegated act is adopted.
