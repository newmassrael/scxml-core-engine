// SCE Forge: Auto-generated from Extended SCXML (sce:kind="condition")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CONDITION_RANGE_H
#define SCE_FORGE_CONDITION_RANGE_H

#include <cstdint>
#include <string>

namespace SCE::Generated::ConditionRange {

inline bool conditionRange(uint32_t rpm, uint32_t minRpm, uint32_t maxRpm) {
    return rpm >= minRpm && rpm <= maxRpm;
}

}  // namespace SCE::Generated::ConditionRange

#endif  // SCE_FORGE_CONDITION_RANGE_H
