//! Battery recycled content validation — EU Regulation 2023/1542, Art. 8.
//!
//! Art. 8(2) and 8(3) set minimum recycled content shares for four metals, in
//! the regulation text itself. Both paragraphs require the **Annex VIII**
//! technical documentation to demonstrate the share — Annex VIII is the only
//! annex Art. 8 cross-references.
//!
//! Phase 1 (from 18 Aug 2031) covers **EV batteries, SLI batteries, and
//! industrial batteries with a capacity > 2 kWh** (excluding those with
//! exclusively external storage). **LMT batteries** join only in Phase 2 (from
//! **18 Aug 2036**), at the higher targets. Portable batteries are out of scope.
//!
//! The targets are **finalized law** — they are in the regulation text itself,
//! not in a pending delegated act. However, neither phase is yet in force, so
//! the battery plugin reports an overall status of `NotAssessed` and surfaces
//! any shortfall as an advisory warning rather than a violation. These constants
//! are the single source of truth it will check against once enforcement begins.
//!
//! ## The measurement basis differs by metal
//!
//! Art. 8(2)/(3) measure cobalt, lithium and nickel as the share recovered from
//! waste that is present **in active materials**; lead is measured as the share
//! present **in the battery**. The percentages below therefore do not share a
//! denominator, and a caller must not sum or average them across metals.
//!
//! ## Two duties, not one
//!
//! Art. 8(1) requires documentation of the *actual* shares, with no minimum,
//! from **18 Aug 2028** (industrial > 2 kWh, EV, SLI) and **18 Aug 2033** (LMT)
//! — or 24 months after the Art. 8(1) methodology delegated act enters into
//! force, whichever is later. Annex XIII point 1(e) makes that documentation
//! publicly accessible passport content. See [`art8_declaration_duty_for`].
//!
//! Art. 8(2) and 8(3) then impose *minimum* shares from 18 Aug 2031 and
//! 18 Aug 2036 — fixed dates, unconditional. See [`art8_phase_for`].
//!
//! The two ladders are independent: a battery can owe the declaration without
//! yet owing a minimum.
//!
//! ## Which phase binds is a function of the placing-on-market date
//!
//! [`art8_category_for`] maps a declared battery type and capacity onto an
//! [`Art8Category`]; [`art8_phase_for`] then picks the governing phase from the
//! date the battery was placed on the EU market. Assessment date is never an
//! input — Art. 8 attaches its duties at placing on the market, so deriving the
//! phase from "today" would report batteries lawfully placed before a phase
//! began as short of it from the moment that phase starts.
//!
//! The battery plugin calls both, and reports a missing or unparseable
//! `placedOnMarketDate` as its own finding rather than assuming one.
//!
//! ## Phase 1 — EV + SLI + industrial > 2 kWh, from **18 Aug 2031** (Art. 8(2))
//! | Material | Minimum % | Basis |
//! |----------|-----------|-------|
//! | Cobalt   |      16 % | active materials |
//! | Lead     |      85 % | the battery |
//! | Lithium  |       6 % | active materials |
//! | Nickel   |       6 % | active materials |
//!
//! ## Phase 2 — Phase 1 categories + **LMT**, from **18 Aug 2036** (Art. 8(3))
//! | Material | Minimum % | Basis |
//! |----------|-----------|-------|
//! | Cobalt   |      26 % | active materials |
//! | Lead     |      85 % | the battery |
//! | Lithium  |      12 % | active materials |
//! | Nickel   |      15 % | active materials |

// BOUNDARY-EXCEPTION(taxonomy): "Phase 1" and "Phase 2" are the two stages of
// Regulation (EU) 2023/1542 Art. 8(2) and 8(3), with statutory dates — the
// Regulation's own staging, not a roadmap.
#[cfg(test)]
mod tests;
mod thresholds;

pub use thresholds::*;
