// SCE-MAP: algorithm_bytes_equal:18

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §5.A: free function in `namespace SCE::Generated::<Name>`. No
// STL containers, no exceptions. `bytes` lowers to `std::span<const
// std::uint8_t>` (RFC §5.J.5 emitter table).

#pragma once
#ifndef SCE_FORGE_BYTES_EQUAL_H
#define SCE_FORGE_BYTES_EQUAL_H

#include <cstddef>
#include <cstdint>
#include <span>
namespace SCE::Generated::BytesEqual {

inline bool bytes_equal(std::span<const std::uint8_t> a, std::span<const std::uint8_t> b) {
    if ((a).size() != (b).size()) {
        return false;
    }
    uint32_t i = 0;
    while (i < (a).size()) {
        if (a[i] != b[i]) {
            return false;
        }
        i = i + 1;
    }
    return true;
}

}  // namespace SCE::Generated::BytesEqual

#endif  // SCE_FORGE_BYTES_EQUAL_H
