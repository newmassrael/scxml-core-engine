// SCE-MAP: validator_plausibility_only:2 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_VALIDATOR_PLAUSIBILITY_ONLY_H
#define SCE_FORGE_VALIDATOR_PLAUSIBILITY_ONLY_H

#include <stdint.h>
#include <stdbool.h>
#include <string.h>

typedef struct {
    bool        valid;
    const char *reason;
} validator_plausibility_only_result_t;

static inline validator_plausibility_only_result_t validator_plausibility_only_validate(double voltage, double current) {
    if (!(voltage * current <= 1000.0))
        return (validator_plausibility_only_result_t){false, "plausibility_failed"};
    return (validator_plausibility_only_result_t){true, ""};
}

#endif  /* SCE_FORGE_VALIDATOR_PLAUSIBILITY_ONLY_H */
