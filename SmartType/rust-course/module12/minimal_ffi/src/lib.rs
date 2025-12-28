use std::ffi::CString;
use std::os::raw::{c_char, c_ulong};

/// Safe wrapper around C's strlen for ASCII-only strings (demo only).
pub fn c_strlen(input: &str) -> usize {
    let cstr = CString::new(input).expect("string had interior nulls");
    unsafe { strlen(cstr.as_ptr()) as usize }
}

/// Safe wrapper that copies a Rust byte slice into a C malloc'd buffer and returns the pointer and length.
/// Caller is responsible for freeing with `c_free`.
pub fn to_c_buffer(data: &[u8]) -> (*mut u8, usize) {
    unsafe {
        let ptr = libc_malloc(data.len()) as *mut u8;
        if ptr.is_null() {
            return (std::ptr::null_mut(), 0);
        }
        std::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len());
        (ptr, data.len())
    }
}

/// Free a buffer allocated by `to_c_buffer`.
pub fn c_free(ptr: *mut u8) {
    unsafe {
        libc_free(ptr as *mut _);
    }
}

unsafe extern "C" {
    fn strlen(s: *const c_char) -> c_ulong;
    fn malloc(size: usize) -> *mut std::ffi::c_void;
    fn free(ptr: *mut std::ffi::c_void);
}

unsafe fn libc_malloc(size: usize) -> *mut std::ffi::c_void {
    unsafe { malloc(size) }
}

unsafe fn libc_free(ptr: *mut std::ffi::c_void) {
    if !ptr.is_null() {
        unsafe { free(ptr) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strlen_matches_rust() {
        let s = "hello ffi";
        assert_eq!(c_strlen(s), s.len());
    }

    #[test]
    fn roundtrip_buffer() {
        let data = b"ffi bytes";
        let (ptr, len) = to_c_buffer(data);
        assert!(!ptr.is_null());
        let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
        assert_eq!(slice, data);
        c_free(ptr);
    }
}
