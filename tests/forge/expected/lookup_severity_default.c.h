// SCE-MAP: lookup_severity_default:9

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup") */
/* Runtime: sce_forge_runtime */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_LOOKUP_SEVERITY_DEFAULT_H
#define SCE_FORGE_LOOKUP_SEVERITY_DEFAULT_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

static const int32_t lookup_severity_default_severity_keys[5] = { 100, 200, 300, 400, 500 };
static const int32_t lookup_severity_default_severity_values[5] = { 1, 2, 3, 2, 4 };

static inline int32_t lookup_severity_default_severity(int32_t code) {
    for (size_t _i = 0; _i < 5; ++_i) {
        if (lookup_severity_default_severity_keys[_i] == code) {
            return lookup_severity_default_severity_values[_i];
        }
    }
    return 0;
}

#endif  /* SCE_FORGE_LOOKUP_SEVERITY_DEFAULT_H */
