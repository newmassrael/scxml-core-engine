// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CROSSFILE_VALIDATOR_CONDITION_H
#define SCE_FORGE_CROSSFILE_VALIDATOR_CONDITION_H

#include <cstdint>
#include <string>
#include "condition_threshold.h"

namespace SCE::Generated::CrossfileValidatorCondition {

struct ValidationResult {
    bool valid;
    std::string reason;
};

struct CrossfileValidatorCondition {

    // Imported kinds (cross-file composition)

    ValidationResult validate(double coolantTemp, double oilTemp, double maxTemp) {
        if (!(!SCE::Generated::ConditionThreshold::conditionThreshold(coolantTemp, oilTemp, maxTemp)))
            return {false, "plausibility_failed"};
        return {true, ""};
    }
};

}  // namespace SCE::Generated::CrossfileValidatorCondition

#endif  // SCE_FORGE_CROSSFILE_VALIDATOR_CONDITION_H
