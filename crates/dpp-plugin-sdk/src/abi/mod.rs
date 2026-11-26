//! Low-level linear-memory ABI: allocation, deallocation, and buffer packing
//! across the Wasm host/guest boundary.

mod memory;

pub use memory::*;
