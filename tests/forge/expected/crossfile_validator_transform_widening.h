// SCE-MAP: crossfile_validator_transform_widening:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CROSSFILE_VALIDATOR_TRANSFORM_WIDENING_H
#define SCE_FORGE_CROSSFILE_VALIDATOR_TRANSFORM_WIDENING_H

#include <cstdint>
#include <string>
#include "transform_temperature.h"

namespace SCE::Generated::CrossfileValidatorTransformWidening {

struct ValidationResult {
    bool valid;
    std::string reason;
};

struct CrossfileValidatorTransformWidening {

    // Imported kinds (cross-file composition)

    ValidationResult validate(uint8_t rawByte) {
        if (rawByte > 200)
            return {false, "raw_byte_out_of_range"};
        if (!(SCE::Generated::TransformTemperature::computeTemperature(rawByte) > SCE::Generated::TransformTemperature::computeTemperature(0)))
            return {false, "plausibility_failed"};
        return {true, ""};
    }
};

}  // namespace SCE::Generated::CrossfileValidatorTransformWidening

#endif  // SCE_FORGE_CROSSFILE_VALIDATOR_TRANSFORM_WIDENING_H
