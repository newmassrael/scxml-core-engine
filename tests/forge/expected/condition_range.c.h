// SCE-MAP: condition_range:3

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="condition") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CONDITION_RANGE_H
#define SCE_FORGE_CONDITION_RANGE_H

#include <stdint.h>
#include <stdbool.h>

static inline bool condition_range_check(uint32_t rpm, uint32_t min_rpm, uint32_t max_rpm) {
    return rpm >= min_rpm && rpm <= max_rpm;
}

#endif  /* SCE_FORGE_CONDITION_RANGE_H */
