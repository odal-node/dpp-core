//! GWP100 characterisation factors from EF 3.1 (IPCC AR6).
//!
//! These are *multipliers* that convert kilograms of a substance into
//! kg CO₂-equivalents. They are **not** lifecycle-inventory (LCI) datasets —
//! the IPCC-derived coefficients are freely available and safe to embed in an
//! Apache-2.0 crate.
//!
//! Source: European Commission EF 3.1 method, which adopts IPCC AR6 GWP100
//! values (2021). Primary reference: IPCC AR6 WG-I Table 7.SM.7 (Supplementary
//! Material to Chapter 7, 2021). See also:
//! <https://eplca.jrc.ec.europa.eu/LCDN/developerEF.xhtml>

mod lookup;
#[cfg(test)]
mod tests;

pub use lookup::*;
