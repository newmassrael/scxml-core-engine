// SCE-MAP: crossfile_validator_transform:3 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CROSSFILE_VALIDATOR_TRANSFORM_H
#define SCE_FORGE_CROSSFILE_VALIDATOR_TRANSFORM_H

#include <stdint.h>
#include <stdbool.h>
#include <string.h>
#include "transform_temperature.h"

typedef struct {
    bool        valid;
    const char *reason;
} crossfile_validator_transform_result_t;

static inline crossfile_validator_transform_result_t crossfile_validator_transform_validate(uint16_t raw_temp) {
    if (raw_temp > 4095)
        return (crossfile_validator_transform_result_t){false, "raw_temp_out_of_range"};
    if (!(transform_temperature_compute_temperature(raw_temp) > -40.0 && transform_temperature_compute_temperature(raw_temp) < 200.0))
        return (crossfile_validator_transform_result_t){false, "plausibility_failed"};
    return (crossfile_validator_transform_result_t){true, ""};
}

#endif  /* SCE_FORGE_CROSSFILE_VALIDATOR_TRANSFORM_H */
