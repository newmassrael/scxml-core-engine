// SCE-MAP: crossfile_validator_filter:14 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CROSSFILE_VALIDATOR_FILTER_H
#define SCE_FORGE_CROSSFILE_VALIDATOR_FILTER_H

#include <cstdint>
#include <string>
#include "filter_low_pass.h"

namespace SCE::Generated::CrossfileValidatorFilter {

struct ValidationResult {
    bool valid;
    std::string reason;
};

struct CrossfileValidatorFilter {

    // Imported kinds (cross-file composition)
    ::SCE::Generated::FilterLowPass::FilterLowPass smoother_{};

    ValidationResult validate(double rawSample, double threshold) {
        if (!(smoother_.update(rawSample) < threshold))
            return {false, "plausibility_failed"};
        return {true, ""};
    }
};

}  // namespace SCE::Generated::CrossfileValidatorFilter

#endif  // SCE_FORGE_CROSSFILE_VALIDATOR_FILTER_H
