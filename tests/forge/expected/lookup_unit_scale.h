// SCE-MAP: lookup_unit_scale:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_LOOKUP_UNIT_SCALE_H
#define SCE_FORGE_LOOKUP_UNIT_SCALE_H

#include <cstdint>
#include <optional>
#include "sce/forge/lookup.h"

namespace SCE::Generated::LookupUnitScale {

constexpr int32_t sce_keys[6] = { 1, 2, 3, 4, 5, 6 };
constexpr double sce_values[6] = { 0.001, 0.01, 0.1, 1.0, 10.0, 100.0 };

inline std::optional<double> lookupScale(int32_t unit) {
    return SCE::Forge::lookup(sce_keys, sce_values, unit);
}

}  // namespace SCE::Generated::LookupUnitScale

#endif  // SCE_FORGE_LOOKUP_UNIT_SCALE_H
