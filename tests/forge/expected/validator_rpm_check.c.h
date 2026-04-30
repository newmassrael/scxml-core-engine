/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_VALIDATOR_RPM_CHECK_H
#define SCE_FORGE_VALIDATOR_RPM_CHECK_H

#include <stdint.h>
#include <stdbool.h>
#include <string.h>

typedef struct {
    bool        valid;
    const char *reason;
} validator_rpm_check_result_t;

typedef struct {
    uint16_t prev_rpm_;
} validator_rpm_check_t;

static inline validator_rpm_check_result_t validator_rpm_check_validate(validator_rpm_check_t *_st, uint16_t rpm, const char * engine_state) {
    if (rpm > 8000)
        return (validator_rpm_check_result_t){false, "rpm_out_of_range"};
    {
        uint16_t delta_ = (rpm > _st->prev_rpm_) ? (rpm - _st->prev_rpm_) : (_st->prev_rpm_ - rpm);
        if (delta_ > 500)
            return (validator_rpm_check_result_t){false, "rpm_rate_of_change_exceeded"};
    }
    if (!(rpm == 0 || strcmp(engine_state, "STOP") != 0))
        return (validator_rpm_check_result_t){false, "plausibility_failed"};
    _st->prev_rpm_ = rpm;
    return (validator_rpm_check_result_t){true, ""};
}

#endif  /* SCE_FORGE_VALIDATOR_RPM_CHECK_H */
