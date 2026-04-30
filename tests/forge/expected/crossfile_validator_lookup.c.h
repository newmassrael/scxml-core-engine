/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CROSSFILE_VALIDATOR_LOOKUP_H
#define SCE_FORGE_CROSSFILE_VALIDATOR_LOOKUP_H

#include <stdint.h>
#include <stdbool.h>
#include <string.h>
#include "lookup_severity_default.h"

typedef struct {
    bool        valid;
    const char *reason;
} crossfile_validator_lookup_result_t;

static inline crossfile_validator_lookup_result_t crossfile_validator_lookup_validate(int32_t code) {
    if (code < 0 || code > 1000)
        return (crossfile_validator_lookup_result_t){false, "code_out_of_range"};
    if (!(lookup_severity_default_severity(code) > 0))
        return (crossfile_validator_lookup_result_t){false, "plausibility_failed"};
    return (crossfile_validator_lookup_result_t){true, ""};
}

#endif  /* SCE_FORGE_CROSSFILE_VALIDATOR_LOOKUP_H */
