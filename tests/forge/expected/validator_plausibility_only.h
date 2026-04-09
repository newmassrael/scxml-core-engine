// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_VALIDATOR_PLAUSIBILITY_ONLY_H
#define SCE_FORGE_VALIDATOR_PLAUSIBILITY_ONLY_H

#include <cstdint>
#include <string>

namespace SCE::Generated::ValidatorPlausibilityOnly {

struct ValidationResult {
    bool valid;
    std::string reason;
};

struct ValidatorPlausibilityOnly {

    ValidationResult validate(double voltage, double current) {
        if (!(voltage * current <= 1000.0))
            return {false, "plausibility_failed"};
        return {true, ""};
    }
};

}  // namespace SCE::Generated::ValidatorPlausibilityOnly

#endif  // SCE_FORGE_VALIDATOR_PLAUSIBILITY_ONLY_H