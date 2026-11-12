//! Tests for the unsold-goods module: the disclosure's shape and schemas, and the
//! discard records, their evidence, retention and aggregation.

mod aggregation;
mod apportion;
mod ban;
mod evidence;
mod record;
mod report;
mod retention;
#[cfg(not(target_arch = "wasm32"))]
mod schema;
