//! The level at which a digital product passport is registered, and the
//! higher-level identifiers a registration at that level must carry.
//!
//! **Where this comes from.** Commission Implementing Regulation (EU) 2026/1778
//! lays down the implementation arrangements for the ESPR Art. 13 registry.
//! Its **Art. 8(1)** requires a passport to be registered "at the level
//! specified in the applicable delegated acts (model, batch or item level)",
//! and **Art. 8(3)** resolves a product caught by several Union rules to "the
//! most granular level required".
//!
//! Two linking obligations follow, and they are the reason this is a type
//! rather than a string:
//!
//! - **Art. 8(4)** — where the passport is created at item level, "both batch
//!   and model identifiers shall be linked to that digital product passport
//!   **where batch and model design exist for the product**".
//! - **Art. 8(5)** — where it is created at batch level, "the model identifier
//!   shall be linked … **where model design exists for the product**".
//!
//! Both obligations are conditional on the design existing. Recital (14) is
//! explicit that "for products that are unique by nature, including handmade
//! goods, no batch and model identifiers are required". So absence is a lawful
//! state and is modelled as `None`; what is *not* lawful is claiming an
//! identifier and leaving it blank, or carrying an identifier finer than the
//! level being registered.

mod level;
#[cfg(test)]
mod tests;

pub use level::*;
