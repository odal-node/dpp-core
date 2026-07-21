//! Cross-crate integration tests for Odal Node core.
//!
//! The only library code this crate hosts is [`fixtures`] — shared test
//! fixture builders consumed by the integration tests in `tests/`, which
//! exercise several core crates together (`dpp-domain` + `dpp-crypto` +
//! `dpp-digital-link`). These tests previously lived at the virtual workspace
//! root (`dpp-core/tests/`), where Cargo never compiled or ran them — moving
//! them into a real workspace member makes them part of `cargo test
//! --workspace` and CI.

pub mod fixtures;
