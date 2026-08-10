// SCE-MAP: crossfile_validator_lookup:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CROSSFILE_VALIDATOR_LOOKUP_H
#define SCE_FORGE_CROSSFILE_VALIDATOR_LOOKUP_H

#include <cstdint>
#include <string>
#include "lookup_severity_default.h"

namespace SCE::Generated::CrossfileValidatorLookup {

struct ValidationResult {
    bool valid;
    std::string reason;
};

struct CrossfileValidatorLookup {

    // Imported kinds (cross-file composition)

    ValidationResult validate(int32_t code) {
        if (code < 0 || code > 1000)
            return {false, "code_out_of_range"};
        if (!(SCE::Generated::LookupSeverityDefault::lookupSeverity(code) > 0))
            return {false, "plausibility_failed"};
        return {true, ""};
    }
};

}  // namespace SCE::Generated::CrossfileValidatorLookup

#endif  // SCE_FORGE_CROSSFILE_VALIDATOR_LOOKUP_H
