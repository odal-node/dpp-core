//! Tests for the unsold-goods module: the disclosure's shape and schemas, and the
//! discard records, their evidence, retention and aggregation.

mod aggregation;
mod apportion;
mod ban;
mod common;
mod evidence;
mod exclusion;
mod record;
mod report;
mod retention;
#[cfg(not(target_arch = "wasm32"))]
mod schema;
mod treatment;
