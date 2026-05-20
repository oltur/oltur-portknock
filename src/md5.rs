//! MD5 hashing via the bundled `oltur-cpp-tools` C++ library.
//!
//! The digest is computed by `oltur_md5` in `oltur-cpp-tools/src/md5.cpp`; this
//! module is only the FFI boundary plus a hex formatter. `build.rs` compiles
//! and links the C++ object.

unsafe extern "C" {
    /// Writes the 16-byte MD5 digest of `len` bytes at `data` into `out`.
    fn oltur_md5(data: *const u8, len: usize, out: *mut u8);
}

/// Compute the MD5 digest of `data`.
pub fn digest(data: &[u8]) -> [u8; 16] {
    let mut out = [0u8; 16];
    // SAFETY: `data`/`len` describe a live slice, and `out` is a writable
    // 16-byte buffer — exactly the regions `oltur_md5` reads and writes.
    unsafe {
        oltur_md5(data.as_ptr(), data.len(), out.as_mut_ptr());
    }
    out
}

/// Format a digest as a 32-character lowercase hex string.
pub fn hex(digest: &[u8; 16]) -> String {
    use std::fmt::Write as _;
    let mut s = String::with_capacity(32);
    for byte in digest {
        let _ = write!(s, "{byte:02x}");
    }
    s
}
