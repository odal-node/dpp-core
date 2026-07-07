//! ABI entry-point wrappers called by [`crate::export_plugin!`].

use dpp_plugin_traits::DppSectorPlugin;

use crate::abi;
use crate::codec::{
    calculate_metrics_bytes, describe_bytes, generate_passport_bytes, metadata_bytes,
    validate_bytes,
};

pub fn run_metadata<P: DppSectorPlugin>(plugin: &P) -> u64 {
    abi::write_output(metadata_bytes(plugin))
}

pub fn run_describe<P: DppSectorPlugin>(plugin: &P) -> u64 {
    abi::write_output(describe_bytes(plugin))
}

/// # Safety
/// `ptr`/`len` must describe a host-written input buffer (see [`abi::read_input`]).
pub unsafe fn run_validate<P: DppSectorPlugin>(plugin: &P, ptr: u32, len: u32) -> u64 {
    unsafe { abi::write_output(validate_bytes(plugin, abi::read_input(ptr, len))) }
}

/// # Safety
/// `ptr`/`len` must describe a host-written input buffer (see [`abi::read_input`]).
pub unsafe fn run_calculate_metrics<P: DppSectorPlugin>(plugin: &P, ptr: u32, len: u32) -> u64 {
    unsafe { abi::write_output(calculate_metrics_bytes(plugin, abi::read_input(ptr, len))) }
}

/// # Safety
/// `ptr`/`len` must describe a host-written input buffer (see [`abi::read_input`]).
pub unsafe fn run_generate_passport<P: DppSectorPlugin>(plugin: &P, ptr: u32, len: u32) -> u64 {
    unsafe { abi::write_output(generate_passport_bytes(plugin, abi::read_input(ptr, len))) }
}
