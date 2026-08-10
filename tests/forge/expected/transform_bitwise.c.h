// SCE-MAP: transform_bitwise:3 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_TRANSFORM_BITWISE_H
#define SCE_FORGE_TRANSFORM_BITWISE_H

#include <stdint.h>
#include <stdbool.h>

static inline uint8_t transform_bitwise_compute_high_nibble(uint8_t byte) {
    return byte >> 4 & 0x0F;
}

static inline uint8_t transform_bitwise_compute_low_nibble(uint8_t byte) {
    return byte & 0x0F;
}

#endif  /* SCE_FORGE_TRANSFORM_BITWISE_H */
