//! Pure glue (host-testable, no linear-memory side effects): JSON
//! encode/decode between a [`DppProductGroupPlugin`] and the ABI byte buffers.

mod bytes;

pub use bytes::*;
