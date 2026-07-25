//! Electronics spare parts availability rules — EU Ecodesign Regulation (ESPR).
//!
//! ## Current schema state
//! The electronics schema v1.0.0 carries `sparePartsAvailable: bool` — a binary
//! declaration. No availability *period* field is defined yet. There are therefore
//! no cross-field rules to implement until the schema gains a period field.
//!
//! ## Regulatory status by product category
//! Minimum spare-parts availability periods are set in **product-specific**
//! ecodesign implementing regulations, not in the ESPR framework regulation itself.
//! Status by category as of 2026:
//!
//! | Category                    | Minimum period | Source regulation      |
//! |-----------------------------|----------------|------------------------|
//! | Washing machines / dryers   | 10 years       | EU 2019/2022 (in force)|
//! | Dishwashers                 | 10 years       | EU 2019/2022 (in force)|
//! | Refrigerators / freezers    | 7–10 years     | EU 2019/2019 (in force)|
//! | Displays / TVs              | 7–10 years     | EU 2019/2021 (in force)|
//! | Smartphones / laptops       | pending        | ESPR delegated act TBD |
//! | Servers                     | pending        | ESPR delegated act TBD |
//! | All other categories        | pending        | ESPR delegated act TBD |
//!
//! These period minimums apply to those pre-ESPR ecodesign regulations. No ESPR
//! delegated act for an electronics DPP has been adopted; under the ESPR working
//! plan (COM(2025) 187 final) electronics is addressed through horizontal
//! repairability and EEE-recyclability measures rather than as a product group.
//! Reg. 2023/1670 does set requirements for smartphones and slate tablets; those
//! are not yet reflected in this module.
//!
//! ## Placeholder note
//! Once the ESPR delegated act adds a `sparePartsAvailabilityYears` field to the
//! electronics schema, implement `validate_spare_parts_period(years, category)`
//! here using the category-keyed minimum periods above.

// Placeholder — rules to be implemented once the electronics schema carries
// a structured spare-parts availability period and the ESPR delegated act
// specifies minimum periods per product category.
