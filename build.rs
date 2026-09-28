//! Compiles the bundled C++ — the pure `oltur-cpp-tools` MD5 library plus the
//! cxx bridge glue — and links it into the `portknock` binary.
//!
//! `cxx-build` generates the FFI shim from the `#[cxx::bridge]` module in
//! `src/md5.rs` and drives the system C++ compiler (overridable via `$CXX`).

fn main() {
    cxx_build::bridge("src/md5.rs")
        .file("oltur-cpp-tools/src/md5.cpp")
        .file("bridge/md5_bridge.cpp")
        .include("oltur-cpp-tools/include")
        .include("bridge")
        .std("c++17")
        .compile("oltur_cpp_tools");

    // cxx-build tracks `src/md5.rs` and the `.cpp` files; headers need flagging.
    println!("cargo:rerun-if-changed=oltur-cpp-tools/include/oltur_cpp_tools.h");
    println!("cargo:rerun-if-changed=bridge/md5_bridge.h");
}
