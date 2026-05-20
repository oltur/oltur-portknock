//! Compiles the bundled `oltur-cpp-tools` C++ library and links it into the
//! `portknock` binary.
//!
//! The system C++ compiler is invoked directly (overridable via the `CXX`
//! environment variable), so the build pulls in no extra Cargo crates.

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let cpp = manifest.join("oltur-cpp-tools");
    let source = cpp.join("src/md5.cpp");
    let header = cpp.join("include/oltur_cpp_tools.h");
    let include = cpp.join("include");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let object = out_dir.join("md5.o");
    let archive = out_dir.join("liboltur_cpp_tools.a");

    // Rebuild only when the C++ sources change.
    println!("cargo:rerun-if-changed={}", source.display());
    println!("cargo:rerun-if-changed={}", header.display());

    let cxx = env::var("CXX").unwrap_or_else(|_| "c++".to_string());
    run(
        Command::new(&cxx)
            .args([
                "-std=c++17",
                "-O2",
                "-fPIC",
                "-fno-exceptions",
                "-fno-rtti",
                "-c",
            ])
            .arg(&source)
            .arg("-I")
            .arg(&include)
            .arg("-o")
            .arg(&object),
        "C++ compilation (is a C++ compiler installed? override it with $CXX)",
    );
    run(
        Command::new("ar").arg("rcs").arg(&archive).arg(&object),
        "archiving liboltur_cpp_tools.a",
    );

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=oltur_cpp_tools");
}

/// Run one build step, aborting the build with a clear message if it fails.
fn run(cmd: &mut Command, step: &str) {
    let status = cmd
        .status()
        .unwrap_or_else(|e| panic!("failed to start {step}: {e}"));
    assert!(status.success(), "{step} failed");
}
