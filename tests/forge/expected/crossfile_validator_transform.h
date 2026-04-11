// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CROSSFILE_VALIDATOR_TRANSFORM_H
#define SCE_FORGE_CROSSFILE_VALIDATOR_TRANSFORM_H

#include <cstdint>
#include <string>
#include "transform_temperature.h"

namespace SCE::Generated::CrossfileValidatorTransform {

struct ValidationResult {
    bool valid;
    std::string reason;
};

struct CrossfileValidatorTransform {

    // Imported kinds (cross-file composition)

    ValidationResult validate(uint16_t rawTemp) {
        if (rawTemp < 0 || rawTemp > 4095)
            return {false, "raw_temp_out_of_range"};
        if (!(SCE::Generated::TransformTemperature::computeTemperature(rawTemp) > -40.0 && SCE::Generated::TransformTemperature::computeTemperature(rawTemp) < 200.0))
            return {false, "plausibility_failed"};
        return {true, ""};
    }
};

}  // namespace SCE::Generated::CrossfileValidatorTransform

#endif  // SCE_FORGE_CROSSFILE_VALIDATOR_TRANSFORM_H
