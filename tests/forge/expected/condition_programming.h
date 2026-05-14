// SCE-MAP: condition_programming:3

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="condition")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CONDITION_PROGRAMMING_H
#define SCE_FORGE_CONDITION_PROGRAMMING_H

#include <cstdint>
#include <string>

namespace SCE::Generated::ConditionProgramming {

inline bool conditionProgramming(bool engineStop, bool ignition) {
    return engineStop == true && ignition == true;
}

}  // namespace SCE::Generated::ConditionProgramming

#endif  // SCE_FORGE_CONDITION_PROGRAMMING_H
