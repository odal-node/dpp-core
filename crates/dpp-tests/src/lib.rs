//! Cross-crate integration tests for Odal Node core.
//!
//! This crate has no library code of its own. It exists solely to host the
//! integration tests in `tests/`, which exercise several core crates together
//! (`dpp-domain` + `dpp-crypto` + `dpp-digital-link`). These tests previously lived at
//! the virtual workspace root (`dpp-core/tests/`), where Cargo never compiled
//! or ran them — moving them into a real workspace member makes them part of
//! `cargo test --workspace` and CI.
