// SCE-MAP: validator_signed_roc:2

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_VALIDATOR_SIGNED_ROC_H
#define SCE_FORGE_VALIDATOR_SIGNED_ROC_H

#include <cstdint>
#include <string>

namespace SCE::Generated::ValidatorSignedRoc {

struct ValidationResult {
    bool valid;
    std::string reason;
};

struct ValidatorSignedRoc {
    int32_t prevSpeed_ = {};
    double prevAltitude_ = {};

    ValidationResult validate(int32_t speed, double altitude) {
        if (speed < -100 || speed > 500)
            return {false, "speed_out_of_range"};
        if (altitude > 50000.0)
            return {false, "altitude_out_of_range"};
        {
            auto delta = static_cast<int64_t>(speed) - static_cast<int64_t>(prevSpeed_);
            if (delta < 0) delta = -delta;
            if (delta > 50)
                return {false, "speed_rate_of_change_exceeded"};
        }
        {
            double delta = (altitude - prevAltitude_);
            if (delta < 0) delta = -delta;
            if (delta > 100.0)
                return {false, "altitude_rate_of_change_exceeded"};
        }
        prevSpeed_ = speed;
        prevAltitude_ = altitude;
        return {true, ""};
    }
};

}  // namespace SCE::Generated::ValidatorSignedRoc

#endif  // SCE_FORGE_VALIDATOR_SIGNED_ROC_H
