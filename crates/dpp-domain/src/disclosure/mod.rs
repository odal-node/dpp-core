//! The disclosure lattice — who may see which class of field.
//!
//! Regulation (EU) 2023/1542 Art. 77(2) names three audiences and assigns each a
//! set of Annex XIII data points. **It is a lattice, not a ranking**: neither
//! audience contains the other, so no integer ordering can express it.
//!
//! Tier 2, not tier 3, and the tier law is what settles that. These are value
//! types the model itself carries — `Passport::redact` takes an [`Audience`] —
//! so filing them with the access *policy* would put a tier-2 aggregate in the
//! position of importing tier 3. The filter is policy; the vocabulary it filters
//! by is not.

mod audience;
mod class;
#[cfg(test)]
mod tests;

pub use audience::Audience;
pub use class::{Disclosure, PASSPORT_FIELD_DISCLOSURE, disclosure_key};
