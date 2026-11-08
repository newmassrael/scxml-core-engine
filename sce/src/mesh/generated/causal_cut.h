// SCE-GENERATED — DO NOT EDIT
// source-hash: f2f8682669670c7aaedbccb3102dd71d7b187eac29a32d3d20509a5bc070c12e
// SCE-MAP: causal_cut.scxml:22 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: free function in `namespace SCE::Generated::<Name>`. No
// STL containers, no exceptions. `bytes` lowers to `SCE::Forge::Span<const
// std::uint8_t>` (RFC §synth-5-J-5 emitter table, docs/adr/0059).

#pragma once
#ifndef SCE_FORGE_CAUSAL_CUT_H
#define SCE_FORGE_CAUSAL_CUT_H

#include "causal_entry.h"
#include "causal_known.h"
#include "envelope_id.h"
#include <cstddef>
#include <cstdint>
#include <sce/forge/span.h>
#include <vector>

namespace SCE::Generated::CausalCut {

inline std::vector<SCE::Generated::CausalEntry::CausalEntryPayload>
causal_cut(uint32_t members, SCE::Forge::Span<const SCE::Generated::CausalKnown::CausalKnownPayload> known) {
    std::vector<SCE::Generated::EnvelopeId::EnvelopeIdPayload> holders;
    holders.reserve(64);
    bool counted = false;
    for (SCE::Generated::CausalKnown::CausalKnownPayload mark : known) {
        counted = false;
        for (SCE::Generated::EnvelopeId::EnvelopeIdPayload holder : holders) {
            if (holder.hi == mark.holderHi && holder.lo == mark.holderLo) {
                counted = true;
            }
        }
        if (!counted) {
            SCE::Generated::EnvelopeId::EnvelopeIdPayload member{/*hi=*/mark.holderHi, /*lo=*/mark.holderLo};
            holders.push_back(member);
        }
    }
    std::vector<SCE::Generated::CausalEntry::CausalEntryPayload> out;
    out.reserve(64);
    if ((holders).size() >= members) {
        bool done = false;
        uint64_t low = 0;
        uint64_t value = 0;
        for (SCE::Generated::CausalKnown::CausalKnownPayload author : known) {
            done = false;
            for (SCE::Generated::CausalEntry::CausalEntryPayload cut : out) {
                if (cut.hi == author.hi && cut.lo == author.lo) {
                    done = true;
                }
            }
            if (!done) {
                low = author.seq;
                for (SCE::Generated::EnvelopeId::EnvelopeIdPayload peer : holders) {
                    value = 0;
                    for (SCE::Generated::CausalKnown::CausalKnownPayload entry : known) {
                        if (entry.holderHi == peer.hi && entry.holderLo == peer.lo && entry.hi == author.hi &&
                            entry.lo == author.lo) {
                            value = entry.seq;
                        }
                    }
                    if (value < low) {
                        low = value;
                    }
                }
                if (low > 0) {
                    SCE::Generated::CausalEntry::CausalEntryPayload stable{/*hi=*/author.hi, /*lo=*/author.lo,
                                                                           /*seq=*/low};
                    out.push_back(stable);
                }
            }
        }
    }
    return out;
}

}  // namespace SCE::Generated::CausalCut

#endif  // SCE_FORGE_CAUSAL_CUT_H
