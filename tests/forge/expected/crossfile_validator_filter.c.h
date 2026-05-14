// SCE-MAP: crossfile_validator_filter:14

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CROSSFILE_VALIDATOR_FILTER_H
#define SCE_FORGE_CROSSFILE_VALIDATOR_FILTER_H

#include <stdint.h>
#include <stdbool.h>
#include <string.h>
#include "filter_low_pass.h"

typedef struct {
    bool        valid;
    const char *reason;
} crossfile_validator_filter_result_t;

typedef struct {
    /* Imported kind members (cross-file composition).
       Mirrors the procedure host's state struct layout — each stateful
       import (codec/filter/...) is embedded by-value so the rename
       map's `_st->{member}.{field}` expansion lands on a real slot and
       the C11 ImportLowering can prepend `&_st->{member}` for method
       dispatch (filter.update, future codec methods, ...). */
    filter_low_pass_t smoother_;
} crossfile_validator_filter_t;

static inline crossfile_validator_filter_result_t crossfile_validator_filter_validate(crossfile_validator_filter_t *_st, double raw_sample, double threshold) {
    if (!(filter_low_pass_update(&_st->smoother_, raw_sample) < threshold))
        return (crossfile_validator_filter_result_t){false, "plausibility_failed"};
    return (crossfile_validator_filter_result_t){true, ""};
}

#endif  /* SCE_FORGE_CROSSFILE_VALIDATOR_FILTER_H */
