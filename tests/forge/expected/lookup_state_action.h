// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_LOOKUP_STATE_ACTION_H
#define SCE_FORGE_LOOKUP_STATE_ACTION_H

#include <cstdint>
#include <optional>
#include "sce/forge/lookup.h"

namespace SCE::Generated::LookupStateAction {

constexpr int32_t KEYS[4] = { 0, 1, 2, 3 };
constexpr int32_t VALUES[4] = { 10, 20, 30, 40 };

inline std::optional<int32_t> lookupAction(int32_t state) {
    return SCE::Forge::lookup(KEYS, VALUES, state);
}

}  // namespace SCE::Generated::LookupStateAction

#endif  // SCE_FORGE_LOOKUP_STATE_ACTION_H
