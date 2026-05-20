// MD5 (RFC 1321) — a from-scratch implementation with no dependencies beyond
// libc's memcpy, compiled with -fno-exceptions -fno-rtti so the object links
// without the C++ runtime.
//
// Efficiency: full 64-byte blocks are hashed straight from the caller's buffer
// with no intermediate copy; only the final partial block is copied so the
// padding can be appended. The work is therefore one linear pass over the
// input plus at most one extra block.

#include "oltur_cpp_tools.h"

#include <cstring>

namespace {

inline uint32_t rotl(uint32_t x, int c) {
    return (x << c) | (x >> (32 - c));
}

// Per-round left-rotate amounts.
const int kShift[64] = {
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22,
    5, 9,  14, 20, 5, 9,  14, 20, 5, 9,  14, 20, 5, 9,  14, 20,
    4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23,
    6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
};

// kConst[i] = floor(2^32 * abs(sin(i + 1))), the standard MD5 constants.
const uint32_t kConst[64] = {
    0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a,
    0xa8304613, 0xfd469501, 0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be,
    0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821, 0xf61e2562, 0xc040b340,
    0x265e5a51, 0xe9b6c7aa, 0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
    0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed, 0xa9e3e905, 0xfcefa3f8,
    0x676f02d9, 0x8d2a4c8a, 0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c,
    0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70, 0x289b7ec6, 0xeaa127fa,
    0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
    0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92,
    0xffeff47d, 0x85845dd1, 0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1,
    0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
};

// Mix one 64-byte block into the running state.
void process_block(uint32_t state[4], const uint8_t block[64]) {
    uint32_t m[16];
    for (int i = 0; i < 16; ++i) {
        m[i] = static_cast<uint32_t>(block[i * 4]) |
               (static_cast<uint32_t>(block[i * 4 + 1]) << 8) |
               (static_cast<uint32_t>(block[i * 4 + 2]) << 16) |
               (static_cast<uint32_t>(block[i * 4 + 3]) << 24);
    }

    uint32_t a = state[0], b = state[1], c = state[2], d = state[3];
    for (int i = 0; i < 64; ++i) {
        uint32_t f;
        int g;
        if (i < 16) {
            f = (b & c) | (~b & d);
            g = i;
        } else if (i < 32) {
            f = (d & b) | (~d & c);
            g = (5 * i + 1) & 15;
        } else if (i < 48) {
            f = b ^ c ^ d;
            g = (3 * i + 5) & 15;
        } else {
            f = c ^ (b | ~d);
            g = (7 * i) & 15;
        }
        uint32_t prev_a = a;
        a = d;
        d = c;
        c = b;
        b = b + rotl(prev_a + f + kConst[i] + m[g], kShift[i]);
    }

    state[0] += a;
    state[1] += b;
    state[2] += c;
    state[3] += d;
}

} // namespace

extern "C" void oltur_md5(const uint8_t *data, size_t len, uint8_t out[16]) {
    uint32_t state[4] = {0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476};

    // Hash every whole block directly from the input.
    size_t full = len & ~static_cast<size_t>(63);
    for (size_t off = 0; off < full; off += 64) {
        process_block(state, data + off);
    }

    // Build the padded final block(s) from the leftover tail. Padding is a
    // 0x80 byte, zeros, then the 64-bit little-endian bit length — landing on
    // a 64-byte boundary, which needs one extra block when the tail is large.
    uint8_t tail[128];
    size_t rem = len - full;
    if (rem > 0) {
        memcpy(tail, data + full, rem);
    }
    tail[rem] = 0x80;
    size_t length_at = (rem < 56) ? 56 : 120;
    for (size_t i = rem + 1; i < length_at; ++i) {
        tail[i] = 0;
    }

    uint64_t bits = static_cast<uint64_t>(len) * 8;
    for (int i = 0; i < 8; ++i) {
        tail[length_at + i] = static_cast<uint8_t>(bits >> (8 * i));
    }

    size_t total = length_at + 8; // 64 or 128
    for (size_t off = 0; off < total; off += 64) {
        process_block(state, tail + off);
    }

    // Emit each state word little-endian.
    for (int i = 0; i < 4; ++i) {
        out[i * 4] = static_cast<uint8_t>(state[i]);
        out[i * 4 + 1] = static_cast<uint8_t>(state[i] >> 8);
        out[i * 4 + 2] = static_cast<uint8_t>(state[i] >> 16);
        out[i * 4 + 3] = static_cast<uint8_t>(state[i] >> 24);
    }
}
