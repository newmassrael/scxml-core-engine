// SCE-MAP: condition_threshold:3 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="condition") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CONDITION_THRESHOLD_H
#define SCE_FORGE_CONDITION_THRESHOLD_H

#include <stdint.h>
#include <stdbool.h>

static inline bool condition_threshold_check(double coolant_temp, double oil_temp, double max_temp) {
    return coolant_temp > max_temp || oil_temp > max_temp;
}

#endif  /* SCE_FORGE_CONDITION_THRESHOLD_H */
