// SCE-MAP: transform_multi_output:3 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_TRANSFORM_MULTI_OUTPUT_H
#define SCE_FORGE_TRANSFORM_MULTI_OUTPUT_H

#include <stdint.h>
#include <stdbool.h>

static inline double transform_multi_output_compute_fahrenheit(double celsius) {
    return celsius * 9.0 / 5.0 + 32.0;
}

static inline double transform_multi_output_compute_kelvin(double celsius) {
    return celsius + 273.15;
}

#endif  /* SCE_FORGE_TRANSFORM_MULTI_OUTPUT_H */
