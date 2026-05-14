// SCE-MAP: lookup_severity_default:9

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_LOOKUP_SEVERITY_DEFAULT_H
#define SCE_FORGE_LOOKUP_SEVERITY_DEFAULT_H

#include <cstdint>
#include <optional>
#include "sce/forge/lookup.h"

namespace SCE::Generated::LookupSeverityDefault {

constexpr int32_t KEYS[5] = { 100, 200, 300, 400, 500 };
constexpr int32_t VALUES[5] = { 1, 2, 3, 2, 4 };

inline int32_t lookupSeverity(int32_t code) {
    return SCE::Forge::lookup(KEYS, VALUES, code).value_or(0);
}

}  // namespace SCE::Generated::LookupSeverityDefault

#endif  // SCE_FORGE_LOOKUP_SEVERITY_DEFAULT_H
