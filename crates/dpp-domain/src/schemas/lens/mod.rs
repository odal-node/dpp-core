//! Schema upcast lenses: pure, versioned, deterministic `v_n → v_m` transforms
//! applied at *read time*.
//!
//! Signed passports are immutable; delegated acts are not. When a product group schema
//! gains a new version, existing signed records must stay byte-identical (their
//! signatures depend on it) yet remain consumable by new-version readers. A lens
//! transforms a record's product group data from the version it was written against up
//! to a newer one, **without touching the canonical signed original** — the
//! derived view carries honest provenance (`derived`, `lens_chain`, `lossy`) and
//! is never presented as the original signature.
//!
//! Only **upcast** (old → new) is supported: the past can read the future never.
//! Lenses are law-adjacent artifacts — each carries the regulatory change that
//! motivated it. They start as Rust impls compiled into core (versioned with the
//! schemas they bridge); an expression/bundle-delivered form can come later.

mod builtin;
#[cfg(test)]
mod builtin_tests;
mod derived_view;

mod registry;
#[cfg(test)]
mod tests;
mod transform;
mod upcast_error;

pub use derived_view::DerivedView;
pub use registry::LensRegistry;
pub use transform::{Lens, LensError};
pub use upcast_error::UpcastError;
