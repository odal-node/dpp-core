//! `PassportRepository` — port for all DPP persistence operations.
//!
//! No physical delete is exposed by design: ESPR retention obligations prohibit
//! removing published passports for the applicable retention period (typically
//! 10–15 years per product group delegated act).
//!
//! # Art. 78(d) — what an implementor may do with this data
//!
//! Regulation (EU) 2023/1542 Art. 78(d): where passport data is stored or
//! otherwise processed by operators authorised to act on behalf of the
//! responsible economic operator, those operators *"shall not be allowed to
//! sell, re-use or process such data, in whole or in part, beyond what is
//! necessary for the provision of the relevant storing or processing
//! services"*.
//!
//! This port is the primary surface that constraint applies to, and [`list`] and
//! [`count`] are the parts of it that see more than one passport at a time. An
//! implementation backing a **hosted** node is a processor in the Art. 78(d)
//! sense, and may use those methods only to serve the operator's own requests —
//! not to derive cross-customer benchmarks, train models, or produce analytics
//! the operator did not ask for.
//!
//! The prohibition is on the *processor*, not the operator: an operator
//! analysing its own passports is doing nothing this article restricts.
//!
//! [`list`]: PassportRepository::list
//! [`count`]: PassportRepository::count

mod port;
mod protected_fields;
#[cfg(test)]
mod protected_fields_tests;
#[cfg(test)]
mod tests;

pub use port::PassportRepository;
pub use protected_fields::PROTECTED_PATCH_FIELDS;
