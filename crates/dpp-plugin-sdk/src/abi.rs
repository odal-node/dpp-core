//! Low-level linear-memory ABI: allocation, deallocation, and buffer packing
//! across the Wasm host/guest boundary.

use std::alloc::{Layout, alloc as mem_alloc, dealloc as mem_dealloc};

/// Allocate `len` bytes in the module's linear memory and return the
/// pointer as a `u32`. Returns `0` for a zero-length request.
///
/// Returns `0` (allocation failure) rather than panicking on an oversized
/// request (`len ≥ isize::MAX`), and never hands out a *truncated* pointer: on
/// a 64-bit host (e.g. a plugin's own native test build) a real heap address
/// does not fit in `u32`, so such an allocation is freed and `0` returned. On
/// wasm32 every address fits, so that guard is inert.
#[must_use]
pub fn host_alloc(len: u32) -> u32 {
    if len == 0 {
        return 0;
    }
    let Ok(layout) = Layout::from_size_align(len as usize, 1) else {
        return 0;
    };
    // SAFETY: `layout` has non-zero size; a null return is handled by the host.
    let ptr = unsafe { mem_alloc(layout) } as usize;
    if ptr > u32::MAX as usize {
        // SAFETY: `ptr`/`layout` are exactly the allocation just returned.
        unsafe { mem_dealloc(ptr as *mut u8, layout) };
        return 0;
    }
    ptr as u32
}

/// Free a buffer previously returned by [`host_alloc`] (or packed into a
/// `-> u64` ABI return). No-op for null pointers, zero length, or an
/// oversized length that could not have been allocated.
pub fn host_dealloc(ptr: u32, len: u32) {
    if ptr == 0 || len == 0 {
        return;
    }
    let Ok(layout) = Layout::from_size_align(len as usize, 1) else {
        return;
    };
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
    // A 32-bit ABI pointer can only address real memory on a 32-bit target
    // (wasm32). On a 64-bit host the value is a truncated address, so never
    // dereference it — return empty for anything but the len==0 case.
    if len == 0 || !cfg!(target_pointer_width = "32") {
        return &[];
    }
    // SAFETY: on wasm32 `ptr`/`len` describe a host-written allocation.
    unsafe { std::slice::from_raw_parts(ptr as *const u8, len as usize) }
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
    let out_ptr = boxed.as_mut_ptr() as usize;
    if out_ptr > u32::MAX as usize {
        // 64-bit host: the address can't be represented in the 32-bit ABI. Drop
        // the buffer (no leak) and return a null pointer with the exact length.
        // On wasm32 every address fits, so this branch is inert.
        return u64::from(out_len);
    }
    std::mem::forget(boxed);
    ((out_ptr as u64) << 32) | u64::from(out_len)
}
