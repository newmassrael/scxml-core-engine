// SCE-MAP: lookup_single_default:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_LOOKUP_SINGLE_DEFAULT_H
#define SCE_FORGE_LOOKUP_SINGLE_DEFAULT_H

#include <cstdint>

namespace SCE::Generated::LookupSingleDefault {

enum class Quality { NONE, LOW, MEDIUM, HIGH };

inline Quality lookupQuality(uint8_t level) {
    switch (level) {
        case 3:
            return Quality::HIGH;
        case 1:
            return Quality::LOW;
        case 2:
            return Quality::MEDIUM;
        case 0:
            return Quality::NONE;
        default: return Quality::NONE;
    }
}

}  // namespace SCE::Generated::LookupSingleDefault

#endif  // SCE_FORGE_LOOKUP_SINGLE_DEFAULT_H
