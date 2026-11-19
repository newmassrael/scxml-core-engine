// SCE-GENERATED — DO NOT EDIT
// source-hash: d3495fb62176332a10644d56c21a8821d7d8d4139f8e29b449e0d708325991a4
// source: causal_lacks.scxml
// SCE-MAP: causal_lacks.scxml:16 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: free function in `namespace SCE::Generated::<Name>`. No
// STL containers, no exceptions. `bytes` lowers to `SCE::Forge::Span<const
// std::uint8_t>` (RFC §synth-5-J-5 emitter table, docs/adr/0059).

#pragma once
#ifndef SCE_FORGE_CAUSAL_LACKS_H
#define SCE_FORGE_CAUSAL_LACKS_H

#include "causal_entry.h"
#include <cstddef>
#include <cstdint>
#include <sce/forge/span.h>

namespace SCE::Generated::CausalLacks {

inline bool causal_lacks(SCE::Forge::Span<const SCE::Generated::CausalEntry::CausalEntryPayload> vector,
                         SCE::Generated::CausalEntry::CausalEntryPayload dot) {
    uint64_t seen = 0;
    for (SCE::Generated::CausalEntry::CausalEntryPayload entry : vector) {
        if (entry.hi == dot.hi && entry.lo == dot.lo) {
            seen = entry.seq;
        }
    }
    return dot.seq > seen;
}

}  // namespace SCE::Generated::CausalLacks

#endif  // SCE_FORGE_CAUSAL_LACKS_H
