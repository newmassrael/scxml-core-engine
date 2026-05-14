// SCE-MAP: crossfile_validator_condition:3

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CROSSFILE_VALIDATOR_CONDITION_H
#define SCE_FORGE_CROSSFILE_VALIDATOR_CONDITION_H

#include <stdint.h>
#include <stdbool.h>
#include <string.h>
#include "condition_threshold.h"

typedef struct {
    bool        valid;
    const char *reason;
} crossfile_validator_condition_result_t;

static inline crossfile_validator_condition_result_t crossfile_validator_condition_validate(double coolant_temp, double oil_temp, double max_temp) {
    if (!(!condition_threshold_check(coolant_temp, oil_temp, max_temp)))
        return (crossfile_validator_condition_result_t){false, "plausibility_failed"};
    return (crossfile_validator_condition_result_t){true, ""};
}

#endif  /* SCE_FORGE_CROSSFILE_VALIDATOR_CONDITION_H */
