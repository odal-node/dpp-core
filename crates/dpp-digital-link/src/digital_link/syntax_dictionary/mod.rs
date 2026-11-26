//! The GS1 Barcode Syntax Dictionary, parsed into an Application Identifier table.
//!
//! The table is **derived from GS1's published dictionary**, never hand-written.
//! Its central fact is per-AI: whether the AI is *pre-defined length* — GS1's
//! `*` flag, meaning no FNC1 separator follows it — or variable length, running
//! to the next separator. Get one wrong and a scanned element string is
//! silently truncated or over-read, producing an identifier that still looks
//! plausible.
//!
//! Provenance, licence and update procedure: `data/README.md`.
//!
//! ## Entry syntax
//!
//! ```text
//! AIs  [Flags]  Specification  [Attributes...]  [# Title]
//!
//! 01         *?  N14,csum,gcppos2   ex=255,37 dlpkey=22,10,21|235   # GTIN
//! 10          ?  X..20              req=01,02,03,8006,8026          # BATCH/LOT
//! 3100-3105  *?  N6                 req=01,02 ex=310n               # NET WEIGHT (kg)
//! 253         ?  N13,csum,gcppos1 [X..17]   dlpkey                  # GDTI
//! ```
//!
//! - **AIs** — one AI, or an inclusive range (`3100-3105`), expanded here.
//! - **Flags** — `*` pre-defined length; `?` permitted as a Digital Link data
//!   attribute. Other flag characters are reserved and ignored.
//! - **Specification** — whitespace-separated components, each `Type[,linter…]`
//!   where the type is `N`/`X`/`Y`/`Z` plus `5` (exactly) or `..20` (up to).
//!   `[…]` marks an optional component. Only the final component may vary.
//! - **Attributes** — `dlpkey` marks a Digital Link primary key.
//!
//! ## What this does not do
//!
//! The dictionary names a *linter* per component (`csum`, `gcppos2`, …). This
//! module reads the components, their lengths, character sets and linter names,
//! and the flags. `value.rs` applies five linters from them (the check digit, the
//! check character pair, the zero filler and the GS1 Company Prefix's leading
//! digits); the rest are not run, and nothing here supports a claim of full GS1
//! validation.

mod ai_spec;
#[cfg(test)]
mod tests;

pub use ai_spec::*;
