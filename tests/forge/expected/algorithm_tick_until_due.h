// SCE-MAP: algorithm_tick_until_due:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: free function in `namespace SCE::Generated::<Name>`. No
// STL containers, no exceptions. `bytes` lowers to `SCE::Forge::Span<const
// std::uint8_t>` (RFC §synth-5-J-5 emitter table, docs/adr/0059).

#pragma once
#ifndef SCE_FORGE_ALGORITHM_TICK_UNTIL_DUE_H
#define SCE_FORGE_ALGORITHM_TICK_UNTIL_DUE_H

#include <cstddef>
#include <cstdint>
#include <sce/forge/algorithm.h>
namespace SCE::Generated::AlgorithmTickUntilDue {

inline uint32_t algorithm_tick_until_due(uint32_t now, uint32_t last, uint32_t period) {
    return SCE::Forge::Saturating::sub<std::uint32_t>(period, SCE::Forge::Wrapping::sub<std::uint32_t>(now, last));
}

}  // namespace SCE::Generated::AlgorithmTickUntilDue

#endif  // SCE_FORGE_ALGORITHM_TICK_UNTIL_DUE_H
