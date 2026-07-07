//! Low-level linear-memory ABI: allocation, deallocation, and buffer packing
//! across the Wasm host/guest boundary.

use std::alloc::{Layout, alloc as mem_alloc, dealloc as mem_dealloc};

/// Allocate `len` bytes in the module's linear memory and return the
/// pointer as a `u32`. Returns `0` for a zero-length request.
#[must_use]
pub fn host_alloc(len: u32) -> u32 {
    if len == 0 {
        return 0;
    }
    let layout = Layout::from_size_align(len as usize, 1).expect("valid layout");
    // SAFETY: `layout` has non-zero size; a null return is handled by the host.
    unsafe { mem_alloc(layout) as u32 }
}

/// Free a buffer previously returned by [`host_alloc`] (or packed into a
/// `-> u64` ABI return). No-op for null pointers or zero length.
pub fn host_dealloc(ptr: u32, len: u32) {
    if ptr == 0 || len == 0 {
        return;
    }
    let layout = Layout::from_size_align(len as usize, 1).expect("valid layout");
    // SAFETY: `ptr`/`len` must describe a buffer from `host_alloc`.
    unsafe { mem_dealloc(ptr as *mut u8, layout) }
}

/// View the host-written input buffer as a byte slice.
///
/// # Safety
///
/// `ptr` and `len` must describe a single allocation written by the host
/// (via `alloc`) that lives for the duration of the returned borrow.
#[must_use]
pub unsafe fn read_input<'a>(ptr: u32, len: u32) -> &'a [u8] {
    unsafe {
        if len == 0 {
            return &[];
        }
        std::slice::from_raw_parts(ptr as *const u8, len as usize)
    }
}

/// Leak `bytes` into linear memory and return the packed
/// `(ptr << 32) | len` the host uses to read and later free it.
///
/// The buffer is shrunk to an exact-size allocation (`capacity == len`,
/// align 1) so that the host's `dealloc(ptr, len)` frees precisely the
/// allocation it was given. Returning a `Vec` directly would leak its
/// (possibly larger) capacity and make `dealloc` a size-mismatched free.
#[must_use]
pub fn write_output(bytes: Vec<u8>) -> u64 {
    let mut boxed = bytes.into_boxed_slice();
    let out_len = boxed.len() as u32;
    let out_ptr = boxed.as_mut_ptr() as usize as u32;
    std::mem::forget(boxed);
    ((out_ptr as u64) << 32) | (out_len as u64)
}
