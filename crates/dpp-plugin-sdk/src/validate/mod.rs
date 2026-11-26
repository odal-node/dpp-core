//! Reusable field validation for product group plugins.
//!
//! [`Validator`] is a fluent collector: each `require_*` / `optional_*` method
//! records a [`PluginFieldError`](dpp_plugin_traits::PluginFieldError) when a check fails and is chainable, so a
//! plugin's `validate_input` reads as a declarative list of field constraints.
//! [`Validator::finish`] reports *all* failures at once rather than stopping at
//! the first — better for surfacing form errors to a manufacturer.
//!
//! The free functions [`num`] and [`str_of`] are convenience readers for
//! `calculate_metrics` bodies, and [`threshold_status`] is the shared
//! "measured value at or under threshold" classification they compare against.

#[cfg(test)]
mod tests;
mod validator;

pub use validator::*;
