//! Battery degradation rules — performance, durability and state-of-health.
//!
//! Two separate regimes, on two separate timelines. Conflating them is the easy
//! mistake here, because only one of them is pending.
//!
//! ## Already in force — declaration duties
//!
//! - **Art. 10(1), since 18 Aug 2024**: rechargeable industrial batteries above
//!   2 kWh, LMT batteries and EV batteries must be accompanied by a document
//!   carrying the electrochemical performance and durability parameters of
//!   **Annex IV Part A**, with Annex IV Part B explaining how they were measured.
//! - **Art. 14(1), since 18 Aug 2024**: the parameters of **Annex VII** must be
//!   held in the battery management system of stationary battery energy storage
//!   systems, LMT batteries and EV batteries.
//!
//! Neither is a threshold. Both are "declare the value", and both are live today.
//!
//! ## Pending — minimum values
//!
//! The values below which a battery is non-compliant come from a delegated act
//! under **Art. 10(5)**, which has not been adopted. There are two of them:
//!
//! | Scope | Act due | Minimum values apply from |
//! |---|---|---|
//! | Industrial > 2 kWh (excl. exclusively external storage) | 18 Feb 2026 | 18 Aug 2027 |
//! | LMT batteries | 18 Feb 2027 | 18 Aug 2028 |
//!
//! Both application dates are conditional: Art. 10(2) and 10(3) read "or 18
//! months after the date of entry into force of the delegated act …, whichever
//! is the latest". Until the act enters into force the effective date is
//! unknown, not merely future.
//!
//! Art. 10(6) is a different power — it lets the Commission *amend the Annex IV
//! parameter list* in light of technical progress. It does not set minimum
//! values, and an earlier version of this module cited it in their place.
//!
//! ## Art. 10(4) — the second-life carve-out
//!
//! Art. 10(1)–(3) do not apply to a battery prepared for re-use, prepared for
//! repurposing, repurposed or remanufactured, where the operator demonstrates it
//! was placed on the market **before** those obligations became applicable. A
//! determination therefore depends on the *original* placing-on-market date, not
//! the date of assessment.
//!
//! ## What Annex VII actually asks for
//!
//! State of health is a parameter set that differs by battery type, not a single
//! percentage:
//!
//! - **EV batteries** — state of certified energy (SOCE).
//! - **Stationary storage and LMT** — remaining capacity; where possible
//!   remaining power capability; where possible remaining round trip efficiency;
//!   evolution of self-discharging rates; where possible ohmic resistance.
//!
//! Annex VII Part B lists the expected-lifetime parameters separately, including
//! date of manufacture and (where appropriate) date of putting into service,
//! energy throughput, capacity throughput, tracking of harmful events, and the
//! number of full equivalent charge-discharge cycles.
//!
//! Which parameter set a battery must report is decided by its category — see
//! [`annex_vii_parameter_set_for`]. The *minimum values* those parameters must
//! reach remain pending under Art. 10(5).

mod parameter_set;
#[cfg(test)]
mod tests;

pub use parameter_set::*;
