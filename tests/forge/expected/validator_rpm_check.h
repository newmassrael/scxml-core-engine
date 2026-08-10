// SCE-MAP: validator_rpm_check:2 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_VALIDATOR_RPM_CHECK_H
#define SCE_FORGE_VALIDATOR_RPM_CHECK_H

#include <cstdint>
#include <string>

namespace SCE::Generated::ValidatorRpmCheck {

struct ValidationResult {
    bool valid;
    std::string reason;
};

struct ValidatorRpmCheck {
    uint16_t prevRpm_ = {};

    ValidationResult validate(uint16_t rpm, const std::string& engineState) {
        if (rpm > 8000)
            return {false, "rpm_out_of_range"};
        {
            uint16_t delta = (rpm > prevRpm_) ? (rpm - prevRpm_) : (prevRpm_ - rpm);
            if (delta > 500)
                return {false, "rpm_rate_of_change_exceeded"};
        }
        if (!(rpm == 0 || engineState != "STOP"))
            return {false, "plausibility_failed"};
        prevRpm_ = rpm;
        return {true, ""};
    }
};

}  // namespace SCE::Generated::ValidatorRpmCheck

#endif  // SCE_FORGE_VALIDATOR_RPM_CHECK_H
