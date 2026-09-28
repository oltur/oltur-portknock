//! MD5 hashing via the bundled `oltur-cpp-tools` C++ library.
//!
//! The boundary is a `cxx` bridge: the `#[cxx::bridge]` module below declares
//! the shared `Digest` struct and the C++ `md5_digest` function. `cxx-build`
//! (in `build.rs`) generates the FFI glue and compiles the C++; the bridge C++
//! lives in `bridge/md5_bridge.{h,cpp}` and adapts the pure C-ABI `oltur_md5`.

#[cxx::bridge(namespace = "oltur")]
mod ffi {
    /// A 16-byte MD5 digest — a struct shared by value between Rust and C++.
    struct Digest {
        bytes: [u8; 16],
    }

    // SAFETY (block-level): asserts the declared C++ function is sound to
    // call. `md5_digest` takes a slice and returns a plain struct — no raw
    // pointers cross here — so each call site stays safe Rust.
    unsafe extern "C++" {
        include!("md5_bridge.h");

        /// Compute the MD5 digest of `data`. Implemented in C++.
        fn md5_digest(data: &[u8]) -> Digest;
    }
}

/// Compute the MD5 digest of `data`.
pub fn digest(data: &[u8]) -> [u8; 16] {
    ffi::md5_digest(data).bytes
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
