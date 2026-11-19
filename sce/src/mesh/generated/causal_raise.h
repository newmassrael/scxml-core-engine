// SCE-GENERATED — DO NOT EDIT
// source-hash: ccc8e0fafb229cb285f809ae6f673953a6b07b436f495cd9dcc97d774ff43e5d
// source: causal_raise.scxml
// SCE-MAP: causal_raise.scxml:21 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: free function in `namespace SCE::Generated::<Name>`. No
// STL containers, no exceptions. `bytes` lowers to `SCE::Forge::Span<const
// std::uint8_t>` (RFC §synth-5-J-5 emitter table, docs/adr/0059).

#pragma once
#ifndef SCE_FORGE_CAUSAL_RAISE_H
#define SCE_FORGE_CAUSAL_RAISE_H

#include "causal_entry.h"
#include <cstddef>
#include <cstdint>
#include <sce/forge/span.h>
#include <vector>

namespace SCE::Generated::CausalRaise {

inline std::vector<SCE::Generated::CausalEntry::CausalEntryPayload>
causal_raise(SCE::Forge::Span<const SCE::Generated::CausalEntry::CausalEntryPayload> vector,
             SCE::Forge::Span<const SCE::Generated::CausalEntry::CausalEntryPayload> other) {
    std::vector<SCE::Generated::CausalEntry::CausalEntryPayload> out;
    out.reserve(64);
    uint64_t top = 0;
    for (SCE::Generated::CausalEntry::CausalEntryPayload mine : vector) {
        top = mine.seq;
        for (SCE::Generated::CausalEntry::CausalEntryPayload theirs : other) {
            if (theirs.hi == mine.hi && theirs.lo == mine.lo && theirs.seq > top) {
                top = theirs.seq;
            }
        }
        SCE::Generated::CausalEntry::CausalEntryPayload raised{/*hi=*/mine.hi, /*lo=*/mine.lo, /*seq=*/top};
        out.push_back(raised);
    }
    bool named = false;
    for (SCE::Generated::CausalEntry::CausalEntryPayload added : other) {
        named = false;
        for (SCE::Generated::CausalEntry::CausalEntryPayload known : vector) {
            if (known.hi == added.hi && known.lo == added.lo) {
                named = true;
            }
        }
        if (!named) {
            out.push_back(added);
        }
    }
    return out;
}

}  // namespace SCE::Generated::CausalRaise

#endif  // SCE_FORGE_CAUSAL_RAISE_H
