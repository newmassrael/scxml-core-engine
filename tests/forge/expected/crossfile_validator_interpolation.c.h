// SCE-MAP: crossfile_validator_interpolation:9

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CROSSFILE_VALIDATOR_INTERPOLATION_H
#define SCE_FORGE_CROSSFILE_VALIDATOR_INTERPOLATION_H

#include <stdint.h>
#include <stdbool.h>
#include <string.h>
#include "interpolation_1d_linear.h"

typedef struct {
    bool        valid;
    const char *reason;
} crossfile_validator_interpolation_result_t;

static inline crossfile_validator_interpolation_result_t crossfile_validator_interpolation_validate(uint16_t rpm) {
    if (rpm < 500 || rpm > 7000)
        return (crossfile_validator_interpolation_result_t){false, "rpm_out_of_range"};
    if (!(interpolation_1d_linear_lookup(rpm) > 200.0))
        return (crossfile_validator_interpolation_result_t){false, "plausibility_failed"};
    return (crossfile_validator_interpolation_result_t){true, ""};
}

#endif  /* SCE_FORGE_CROSSFILE_VALIDATOR_INTERPOLATION_H */
