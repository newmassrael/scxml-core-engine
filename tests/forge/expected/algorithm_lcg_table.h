// SCE-MAP: algorithm_lcg_table:13 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: free function in `namespace SCE::Generated::<Name>`. No
// STL containers, no exceptions. `bytes` lowers to `SCE::Forge::Span<const
// std::uint8_t>` (RFC §synth-5-J-5 emitter table, docs/adr/0059).

#pragma once
#ifndef SCE_FORGE_ALGORITHM_LCG_TABLE_H
#define SCE_FORGE_ALGORITHM_LCG_TABLE_H

#include <array>
#include <cstddef>
#include <cstdint>
namespace SCE::Generated::AlgorithmLcgTable {

inline constexpr std::array<uint32_t, 16> LCG_TABLE = { 12345, 87628868, 71072467, 2332836374, 2726892157, 3908547000, 483019191, 2129828778, 2355140353, 2560230508, 3364893915, 171172990, 3194601925, 4148119648, 316399679, 3004788882 };

inline uint32_t algorithm_lcg_table(uint8_t n) {
    return LCG_TABLE[n & 15];
}

}  // namespace SCE::Generated::AlgorithmLcgTable

#endif  // SCE_FORGE_ALGORITHM_LCG_TABLE_H
