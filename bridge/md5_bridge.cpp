// cxx bridge for oltur-cpp-tools' MD5 — see md5_bridge.h.

#include "md5_bridge.h"

#include "oltur_cpp_tools.h" // the pure C-ABI `oltur_md5`

namespace oltur {

Digest md5_digest(rust::Slice<const std::uint8_t> data) {
    Digest digest{};
    // `oltur_md5` accepts a null pointer only when the length is zero, which
    // is exactly what an empty `rust::Slice` yields — so this is always sound.
    ::oltur_md5(data.data(), data.size(), digest.bytes.data());
    return digest;
}

} // namespace oltur
