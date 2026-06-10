// SCE-MAP: algorithm_bytes_equal:18

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */
/* */
/* RFC §synth-5-A: free function with bounded loops, no allocs, no I/O. */
/* RFC §synth-5-J-5 emitter table: `static T <snake>(...)` with `bytes` */
/* lowered to the borrowed `sce_forge_bytes_view_t` (zero-copy). */

#ifndef SCE_FORGE_BYTES_EQUAL_H
#define SCE_FORGE_BYTES_EQUAL_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include "sce/forge/bytes.h"  /* sce_forge_bytes_view_t */
static inline bool bytes_equal(sce_forge_bytes_view_t a, sce_forge_bytes_view_t b) {
    if ((a).len != (b).len) {
        return false;
    }
    uint32_t i = 0;
    while (i < (a).len) {
        if (a.data[i] != b.data[i]) {
            return false;
        }
        i = i + 1;
    }
    return true;
}

#endif  /* SCE_FORGE_BYTES_EQUAL_H */
