//! Battery Carbon Footprint (CFB) calculator — not yet implemented.
//!
//! Will implement Article 7 of EU Battery Regulation 2023/1542 once the carbon
//! footprint delegated act under that article is adopted. **It has not been.**
//! It was due February 2025 and has slipped repeatedly; as of July 2026 no OJ
//! number exists. The methodology is expected to follow the PEF Category Rules
//! (PEF-CR) for rechargeable batteries.
//!
//! Do not mistake Commission Delegated Regulation (EU) 2025/606 for this act —
//! that one covers recycling efficiency and material recovery rates, a different
//! subject, and is not a basis for carbon footprint calculation.
//!
//! **Status: stub.** This module compiles but all entry points return
//! `CalcError::NotImplemented`. Two things are missing and neither is code:
//!   1. The delegated act's number and its Annex, without which there is no
//!      methodology to implement — only a guess at one.
//!   2. Life-cycle inventory data for the process routes. This crate never
//!      bundles LCI data; it is supplied at runtime through [`FactorProvider`](crate::factor::FactorProvider),
//!      and the datasets that carry it are licensed separately by whoever
//!      operates the calculation.
//!
//! Implementing against a draft would produce a number that looks official and
//! is not, which is worse than an honest `NotImplemented`.
//!
//! ⚠️ COMPLIANCE-PIN PENDING: CFB Delegated Act number ("EU 2025/…") must be
//! confirmed against EUR-Lex before implementation begins. Cite the final OJ number.

mod calculation;
#[cfg(test)]
mod tests;

pub use calculation::*;
