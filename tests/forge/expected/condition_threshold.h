// SCE-MAP: condition_threshold:3

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="condition")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CONDITION_THRESHOLD_H
#define SCE_FORGE_CONDITION_THRESHOLD_H

#include <cstdint>
#include <string>

namespace SCE::Generated::ConditionThreshold {

inline bool conditionThreshold(double coolantTemp, double oilTemp, double maxTemp) {
    return coolantTemp > maxTemp || oilTemp > maxTemp;
}

}  // namespace SCE::Generated::ConditionThreshold

#endif  // SCE_FORGE_CONDITION_THRESHOLD_H
