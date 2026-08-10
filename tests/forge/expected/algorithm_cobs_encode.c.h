// SCE-MAP: algorithm_cobs_encode:32 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */
/* */
/* RFC §synth-5-A: free function with bounded loops, no allocs, no I/O. */
/* RFC §synth-5-J-5 emitter table: `static T <snake>(...)` with `bytes` */
/* lowered to the borrowed `sce_forge_bytes_view_t` (zero-copy). */

#ifndef SCE_FORGE_ALGORITHM_COBS_ENCODE_H
#define SCE_FORGE_ALGORITHM_COBS_ENCODE_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include "sce/forge/bytes.h"  /* sce_forge_bytes_view_t */
typedef struct {
    uint8_t bytes[32];
    size_t len;
    bool ok;
} algorithm_cobs_encode_result_t;

static inline algorithm_cobs_encode_result_t algorithm_cobs_encode(sce_forge_bytes_view_t data) {
    uint16_t n = (data).len;
    algorithm_cobs_encode_result_t out = { .len = 0u, .ok = true };
    uint16_t p = 0;
    bool done = false;
    while (done == false) {
        uint16_t q = p;
        while (q < n && q - p < 254 && data.data[q] != 0) {
            q = q + 1;
        }
        uint16_t run = q - p;
        uint8_t code = run + 1;
        if (out.len < 32u) { out.bytes[out.len++] = (uint8_t)(code); } else { out.ok = false; }
        uint16_t k = p;
        while (k < q) {
            if (out.len < 32u) { out.bytes[out.len++] = (uint8_t)(data.data[k]); } else { out.ok = false; }
            k = k + 1;
        }
        if (q >= n) {
            done = true;
        } else {
            if (run < 254) {
                p = q + 1;
                if (p >= n) {
                    uint8_t last = 1;
                    if (out.len < 32u) { out.bytes[out.len++] = (uint8_t)(last); } else { out.ok = false; }
                    done = true;
                }
            } else {
                p = q;
            }
        }
    }
    return out;
}

#endif  /* SCE_FORGE_ALGORITHM_COBS_ENCODE_H */
