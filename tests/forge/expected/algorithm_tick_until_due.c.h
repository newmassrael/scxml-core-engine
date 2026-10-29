// SCE-MAP: algorithm_tick_until_due:12 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */
/* */
/* RFC §synth-5-A: free function with bounded loops, no allocs, no I/O. */
/* RFC §synth-5-J-5 emitter table: `static T <snake>(...)` with `bytes` */
/* lowered to the borrowed `sce_forge_bytes_view_t` (zero-copy). */

#ifndef SCE_FORGE_ALGORITHM_TICK_UNTIL_DUE_H
#define SCE_FORGE_ALGORITHM_TICK_UNTIL_DUE_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include "sce/forge/algorithm.h"  /* integer operations, SCE_FORGE.md §3.4.1 and §3.4.2 */
static inline uint32_t algorithm_tick_until_due(uint32_t now, uint32_t last, uint32_t period) {
    return sce_forge_saturating_sub_u32(period, sce_forge_wrapping_sub_u32(now, last));
}

#endif  /* SCE_FORGE_ALGORITHM_TICK_UNTIL_DUE_H */
