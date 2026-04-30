// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CROSSFILE_VALIDATOR_INTERPOLATION_H
#define SCE_FORGE_CROSSFILE_VALIDATOR_INTERPOLATION_H

#include <cstdint>
#include <string>
#include "interpolation_1d_linear.h"

namespace SCE::Generated::CrossfileValidatorInterpolation {

struct ValidationResult {
    bool valid;
    std::string reason;
};

struct CrossfileValidatorInterpolation {

    // Imported kinds (cross-file composition)

    ValidationResult validate(uint16_t rpm) {
        if (rpm < 500 || rpm > 7000)
            return {false, "rpm_out_of_range"};
        if (!(SCE::Generated::Interpolation1dLinear::Interpolation1dLinear::lookup(rpm) > 200.0))
            return {false, "plausibility_failed"};
        return {true, ""};
    }
};

}  // namespace SCE::Generated::CrossfileValidatorInterpolation

#endif  // SCE_FORGE_CROSSFILE_VALIDATOR_INTERPOLATION_H
