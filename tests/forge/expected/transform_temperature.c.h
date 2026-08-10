// SCE-MAP: transform_temperature:3 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_TRANSFORM_TEMPERATURE_H
#define SCE_FORGE_TRANSFORM_TEMPERATURE_H

#include <stdint.h>
#include <stdbool.h>

static inline double transform_temperature_compute_temperature(uint16_t raw) {
    return raw * 0.1 - 40.0;
}

#endif  /* SCE_FORGE_TRANSFORM_TEMPERATURE_H */
