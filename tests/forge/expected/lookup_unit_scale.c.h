// SCE-MAP: lookup_unit_scale:6 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup") */
/* Runtime: sce_forge_runtime */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_LOOKUP_UNIT_SCALE_H
#define SCE_FORGE_LOOKUP_UNIT_SCALE_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

static const int32_t lookup_unit_scale_scale_keys[6] = { 1, 2, 3, 4, 5, 6 };
static const double lookup_unit_scale_scale_values[6] = { 0.001, 0.01, 0.1, 1.0, 10.0, 100.0 };

static inline bool lookup_unit_scale_scale(int32_t unit, double *out) {
    for (size_t _i = 0; _i < 6; ++_i) {
        if (lookup_unit_scale_scale_keys[_i] == unit) {
            *out = lookup_unit_scale_scale_values[_i];
            return true;
        }
    }
    return false;
}

#endif  /* SCE_FORGE_LOOKUP_UNIT_SCALE_H */
