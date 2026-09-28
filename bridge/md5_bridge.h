// cxx bridge for oltur-cpp-tools' MD5.
//
// Adapts the pure C-ABI `oltur_md5` to the `rust::Slice` / shared-struct
// signature that the cxx-generated glue calls. This header is `include!`-d by
// the `#[cxx::bridge]` module in `src/md5.rs`; keeping it separate leaves
// `oltur-cpp-tools/` a plain C-ABI library, unaware of cxx.
#pragma once

#include <cstdint>

#include "portknock/src/md5.rs.h" // cxx-generated: declares `oltur::Digest`

namespace oltur {

// Compute the MD5 digest of `data`. Called from Rust through the cxx bridge.
Digest md5_digest(rust::Slice<const std::uint8_t> data);

} // namespace oltur
