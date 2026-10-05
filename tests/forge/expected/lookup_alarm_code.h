// SCE-MAP: lookup_alarm_code:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_LOOKUP_ALARM_CODE_H
#define SCE_FORGE_LOOKUP_ALARM_CODE_H

#include <cstdint>
#include <optional>
#include "sce/forge/lookup.h"

namespace SCE::Generated::LookupAlarmCode {

constexpr int32_t sce_keys[5] = { 100, 200, 300, 400, 500 };
constexpr int32_t sce_values[5] = { 1, 2, 3, 2, 4 };

inline std::optional<int32_t> lookupSeverity(int32_t code) {
    return SCE::Forge::lookup(sce_keys, sce_values, code);
}

}  // namespace SCE::Generated::LookupAlarmCode

#endif  // SCE_FORGE_LOOKUP_ALARM_CODE_H
