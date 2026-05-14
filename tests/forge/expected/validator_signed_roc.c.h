// SCE-MAP: validator_signed_roc:2

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_VALIDATOR_SIGNED_ROC_H
#define SCE_FORGE_VALIDATOR_SIGNED_ROC_H

#include <stdint.h>
#include <stdbool.h>
#include <string.h>

typedef struct {
    bool        valid;
    const char *reason;
} validator_signed_roc_result_t;

typedef struct {
    int32_t prev_speed_;
    double prev_altitude_;
} validator_signed_roc_t;

static inline validator_signed_roc_result_t validator_signed_roc_validate(validator_signed_roc_t *_st, int32_t speed, double altitude) {
    if (speed < -100 || speed > 500)
        return (validator_signed_roc_result_t){false, "speed_out_of_range"};
    if (altitude > 50000.0)
        return (validator_signed_roc_result_t){false, "altitude_out_of_range"};
    {
        int64_t delta_ = (int64_t)speed - (int64_t)_st->prev_speed_;
        if (delta_ < 0) delta_ = -delta_;
        if (delta_ > 50)
            return (validator_signed_roc_result_t){false, "speed_rate_of_change_exceeded"};
    }
    {
        double delta_ = (altitude - _st->prev_altitude_);
        if (delta_ < 0) delta_ = -delta_;
        if (delta_ > 100.0)
            return (validator_signed_roc_result_t){false, "altitude_rate_of_change_exceeded"};
    }
    _st->prev_speed_ = speed;
    _st->prev_altitude_ = altitude;
    return (validator_signed_roc_result_t){true, ""};
}

#endif  /* SCE_FORGE_VALIDATOR_SIGNED_ROC_H */
