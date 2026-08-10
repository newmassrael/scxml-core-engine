// SCE-MAP: validator_range_only:2 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_VALIDATOR_RANGE_ONLY_H
#define SCE_FORGE_VALIDATOR_RANGE_ONLY_H

#include <cstdint>
#include <string>

namespace SCE::Generated::ValidatorRangeOnly {

struct ValidationResult {
    bool valid;
    std::string reason;
};

struct ValidatorRangeOnly {

    ValidationResult validate(double temperature) {
        if (temperature < -40.0 || temperature > 150.0)
            return {false, "temperature_out_of_range"};
        return {true, ""};
    }
};

}  // namespace SCE::Generated::ValidatorRangeOnly

#endif  // SCE_FORGE_VALIDATOR_RANGE_ONLY_H
