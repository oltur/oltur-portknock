//! Compiles the bundled `oltur-cpp-tools` C++ library and links it into the
//! `portknock` binary.
//!
//! The `cc` crate drives the system C++ compiler (still overridable via the
//! `CXX` environment variable) and handles object compilation, archiving, and
//! the `cargo:rustc-link-*` directives.

fn main() {
    // `cc` emits `rerun-if-changed` for the source files it compiles, but not
    // for headers, so flag the public header explicitly.
    println!("cargo:rerun-if-changed=oltur-cpp-tools/include/oltur_cpp_tools.h");

    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .flag_if_supported("-fno-exceptions")
        .flag_if_supported("-fno-rtti")
        .include("oltur-cpp-tools/include")
        .file("oltur-cpp-tools/src/md5.cpp")
        .compile("oltur_cpp_tools");
}
