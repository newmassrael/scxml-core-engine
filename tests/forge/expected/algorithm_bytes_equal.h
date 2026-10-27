// SCE-MAP: algorithm_bytes_equal:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: free function in `namespace SCE::Generated::<Name>`. No
// STL containers, no exceptions. `bytes` lowers to `SCE::Forge::Span<const
// std::uint8_t>` (RFC §synth-5-J-5 emitter table, docs/adr/0059).

#pragma once
#ifndef SCE_FORGE_BYTES_EQUAL_H
#define SCE_FORGE_BYTES_EQUAL_H

#include <cstddef>
#include <cstdint>
#include <sce/forge/span.h>
namespace SCE::Generated::BytesEqual {

inline bool bytes_equal(SCE::Forge::Span<const std::uint8_t> a, SCE::Forge::Span<const std::uint8_t> b) {
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
