// SCE-MAP: lookup_alarm_code:6 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup") */
/* Runtime: sce_forge_runtime */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_LOOKUP_ALARM_CODE_H
#define SCE_FORGE_LOOKUP_ALARM_CODE_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

static const int32_t lookup_alarm_code_severity_keys[5] = { 100, 200, 300, 400, 500 };
static const int32_t lookup_alarm_code_severity_values[5] = { 1, 2, 3, 2, 4 };

static inline bool lookup_alarm_code_severity(int32_t code, int32_t *sce_out) {
    for (size_t sce_i = 0; sce_i < 5; ++sce_i) {
        if (lookup_alarm_code_severity_keys[sce_i] == code) {
            *sce_out = lookup_alarm_code_severity_values[sce_i];
            return true;
        }
    }
    return false;
}

#endif  /* SCE_FORGE_LOOKUP_ALARM_CODE_H */
