// SCE-MAP: crossfile_validator_transform_widening:9 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CROSSFILE_VALIDATOR_TRANSFORM_WIDENING_H
#define SCE_FORGE_CROSSFILE_VALIDATOR_TRANSFORM_WIDENING_H

#include <stdint.h>
#include <stdbool.h>
#include <string.h>
#include "transform_temperature.h"

typedef struct {
    bool        valid;
    const char *reason;
} crossfile_validator_transform_widening_result_t;

static inline crossfile_validator_transform_widening_result_t crossfile_validator_transform_widening_validate(uint8_t raw_byte) {
    if (raw_byte > 200)
        return (crossfile_validator_transform_widening_result_t){false, "raw_byte_out_of_range"};
    if (!(transform_temperature_compute_temperature(raw_byte) > transform_temperature_compute_temperature(0)))
        return (crossfile_validator_transform_widening_result_t){false, "plausibility_failed"};
    return (crossfile_validator_transform_widening_result_t){true, ""};
}

#endif  /* SCE_FORGE_CROSSFILE_VALIDATOR_TRANSFORM_WIDENING_H */
