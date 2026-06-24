//! Care symbol validation — ISO 3758:2012 care labelling.
//!
//! ## Current schema state
//! The textile schema v1.1.0 carries `careInstructions` as a **free-text string**
//! (`"ISO 3758 care symbols or free text care instructions"`). There is no
//! structured array of individual symbols. Cross-field validation of individual
//! care symbol codes is therefore not applicable until the schema is updated to
//! carry a structured representation (e.g. an array of symbol objects).
//!
//! ## ISO 3758:2012 symbol categories (for reference)
//! | Category        | Examples                              |
//! |-----------------|---------------------------------------|
//! | Washing         | 30 °C, 40 °C, 60 °C, 95 °C, hand wash |
//! | Bleaching       | any bleach, non-chlorine only, do not bleach |
//! | Tumble drying   | normal, low heat, do not tumble dry   |
//! | Natural drying  | line dry, drip dry, dry flat, in shade|
//! | Ironing         | low (110 °C), medium (150 °C), high (200 °C), do not iron |
//! | Professional    | dry clean (F, P, W), wet clean        |
//!
//! ## Placeholder note
//! When the textile schema introduces a structured `careSymbols` array field,
//! implement:
//! - `washing_temperature_valid(temp_c: u32) -> bool` — allowed values: 30, 40, 60, 70, 95
//! - `care_treatment_valid(treatment: &str) -> bool` — checks against the ISO 3758 symbol set
//! - Cross-field: if `washingTempC` is declared, a washing symbol must also be present

// Placeholder — rules to be implemented once the textile schema carries
// a structured care-symbol array rather than a free-text string.
