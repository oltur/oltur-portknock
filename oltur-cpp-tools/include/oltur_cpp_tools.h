/* oltur-cpp-tools — small C++ utilities exported with a C ABI.
 *
 * Everything here uses C linkage so it is callable from C and from other
 * languages over FFI. This repository's Rust binary calls `oltur_md5`.
 */
#ifndef OLTUR_CPP_TOOLS_H
#define OLTUR_CPP_TOOLS_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Compute the MD5 digest of a byte array.
 *
 *   data - pointer to `len` input bytes (may be NULL only when len == 0)
 *   len  - number of input bytes
 *   out  - caller-owned buffer; exactly 16 bytes of digest are written
 */
void oltur_md5(const uint8_t *data, size_t len, uint8_t out[16]);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* OLTUR_CPP_TOOLS_H */
