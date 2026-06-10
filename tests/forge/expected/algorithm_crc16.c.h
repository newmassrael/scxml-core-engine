// SCE-MAP: algorithm_crc16:11

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */
/* */
/* RFC §synth-5-A: free function with bounded loops, no allocs, no I/O. */
/* RFC §synth-5-J-5 emitter table: `static T <snake>(...)` with `bytes` */
/* lowered to the borrowed `sce_forge_bytes_view_t` (zero-copy). */

#ifndef SCE_FORGE_ALGORITHM_CRC16_H
#define SCE_FORGE_ALGORITHM_CRC16_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include "sce/forge/bytes.h"  /* sce_forge_bytes_view_t */
static inline uint16_t algorithm_crc16(sce_forge_bytes_view_t data) {
    uint16_t crc = 0xFFFF;
    for (size_t __i = 0; __i < data.len; ++__i) {
        uint8_t b = data.data[__i];
        uint16_t hi = b;
        crc = crc ^ hi << 8;
        uint8_t i = 0;
        while (i < 8) {
            if ((crc & 0x8000) != 0) {
                crc = crc << 1 ^ 0x1021;
            } else {
                crc = crc << 1;
            }
            i = i + 1;
        }
    }
    return crc;
}

#endif  /* SCE_FORGE_ALGORITHM_CRC16_H */
