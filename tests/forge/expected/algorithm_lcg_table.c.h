// SCE-MAP: algorithm_lcg_table:13 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */
/* */
/* RFC §synth-5-A: free function with bounded loops, no allocs, no I/O. */
/* RFC §synth-5-J-5 emitter table: `static T <snake>(...)` with `bytes` */
/* lowered to the borrowed `sce_forge_bytes_view_t` (zero-copy). */

#ifndef SCE_FORGE_ALGORITHM_LCG_TABLE_H
#define SCE_FORGE_ALGORITHM_LCG_TABLE_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
static const uint32_t LCG_TABLE[16] = { 12345, 87628868, 71072467, 2332836374, 2726892157, 3908547000, 483019191, 2129828778, 2355140353, 2560230508, 3364893915, 171172990, 3194601925, 4148119648, 316399679, 3004788882 };

static inline uint32_t algorithm_lcg_table(uint8_t n) {
    return LCG_TABLE[n & 15];
}

#endif  /* SCE_FORGE_ALGORITHM_LCG_TABLE_H */
