// SCE-GENERATED — DO NOT EDIT
// source-hash: f2f8682669670c7aaedbccb3102dd71d7b187eac29a32d3d20509a5bc070c12e
// SCE-MAP: causal_admit.scxml:25 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: free function in `namespace SCE::Generated::<Name>`. No
// STL containers, no exceptions. `bytes` lowers to `SCE::Forge::Span<const
// std::uint8_t>` (RFC §synth-5-J-5 emitter table, docs/adr/0059).

#pragma once
#ifndef SCE_FORGE_CAUSAL_ADMIT_H
#define SCE_FORGE_CAUSAL_ADMIT_H

#include "causal_admission.h"
#include "causal_entry.h"
#include <cstddef>
#include <cstdint>
#include <sce/forge/algorithm.h>
#include <sce/forge/span.h>

namespace SCE::Generated::CausalAdmit {

inline SCE::Forge::AlgorithmResult<SCE::Generated::CausalAdmission::CausalAdmission>
causal_admit(SCE::Forge::Span<const SCE::Generated::CausalEntry::CausalEntryPayload> delivered,
             SCE::Generated::CausalEntry::CausalEntryPayload dot,
             SCE::Forge::Span<const SCE::Generated::CausalEntry::CausalEntryPayload> context) {
    // SCE_FORGE.md §3.4.1: each checked operation records a failure here, and
    // the statement around it returns it before the next statement runs.
    SCE::Forge::AlgorithmFailure sce_failure_;
    uint64_t seen = 0;
    if (sce_failure_.failed()) {
        return SCE::Forge::AlgorithmResult<SCE::Generated::CausalAdmission::CausalAdmission>::failure(
            sce_failure_.error());
    }
    for (SCE::Generated::CausalEntry::CausalEntryPayload mine : delivered) {
        if (const bool sce_cond2_ = mine.hi == dot.hi && mine.lo == dot.lo; sce_failure_.failed()) {
            return SCE::Forge::AlgorithmResult<SCE::Generated::CausalAdmission::CausalAdmission>::failure(
                sce_failure_.error());
        } else if (sce_cond2_) {
            seen = mine.seq;
            if (sce_failure_.failed()) {
                return SCE::Forge::AlgorithmResult<SCE::Generated::CausalAdmission::CausalAdmission>::failure(
                    sce_failure_.error());
            }
        }
    }
    bool covered = true;
    if (sce_failure_.failed()) {
        return SCE::Forge::AlgorithmResult<SCE::Generated::CausalAdmission::CausalAdmission>::failure(
            sce_failure_.error());
    }
    uint64_t have = 0;
    if (sce_failure_.failed()) {
        return SCE::Forge::AlgorithmResult<SCE::Generated::CausalAdmission::CausalAdmission>::failure(
            sce_failure_.error());
    }
    for (SCE::Generated::CausalEntry::CausalEntryPayload needed : context) {
        have = 0;
        if (sce_failure_.failed()) {
            return SCE::Forge::AlgorithmResult<SCE::Generated::CausalAdmission::CausalAdmission>::failure(
                sce_failure_.error());
        }
        for (SCE::Generated::CausalEntry::CausalEntryPayload held : delivered) {
            if (const bool sce_cond3_ = held.hi == needed.hi && held.lo == needed.lo; sce_failure_.failed()) {
                return SCE::Forge::AlgorithmResult<SCE::Generated::CausalAdmission::CausalAdmission>::failure(
                    sce_failure_.error());
            } else if (sce_cond3_) {
                have = held.seq;
                if (sce_failure_.failed()) {
                    return SCE::Forge::AlgorithmResult<SCE::Generated::CausalAdmission::CausalAdmission>::failure(
                        sce_failure_.error());
                }
            }
        }
        if (const bool sce_cond2_ = (needed.hi != dot.hi || needed.lo != dot.lo) && needed.seq > have;
            sce_failure_.failed()) {
            return SCE::Forge::AlgorithmResult<SCE::Generated::CausalAdmission::CausalAdmission>::failure(
                sce_failure_.error());
        } else if (sce_cond2_) {
            covered = false;
            if (sce_failure_.failed()) {
                return SCE::Forge::AlgorithmResult<SCE::Generated::CausalAdmission::CausalAdmission>::failure(
                    sce_failure_.error());
            }
        }
    }
    SCE::Generated::CausalAdmission::CausalAdmission result = SCE::Generated::CausalAdmission::CausalAdmission::Hold;
    if (sce_failure_.failed()) {
        return SCE::Forge::AlgorithmResult<SCE::Generated::CausalAdmission::CausalAdmission>::failure(
            sce_failure_.error());
    }
    if (const bool sce_cond1_ = dot.seq <= seen; sce_failure_.failed()) {
        return SCE::Forge::AlgorithmResult<SCE::Generated::CausalAdmission::CausalAdmission>::failure(
            sce_failure_.error());
    } else if (sce_cond1_) {
        result = SCE::Generated::CausalAdmission::CausalAdmission::Drop;
        if (sce_failure_.failed()) {
            return SCE::Forge::AlgorithmResult<SCE::Generated::CausalAdmission::CausalAdmission>::failure(
                sce_failure_.error());
        }
    }
    if (const bool sce_cond1_ = dot.seq == SCE::Forge::Checked::add<std::uint64_t>(sce_failure_, seen, 1) && covered;
        sce_failure_.failed()) {
        return SCE::Forge::AlgorithmResult<SCE::Generated::CausalAdmission::CausalAdmission>::failure(
            sce_failure_.error());
    } else if (sce_cond1_) {
        result = SCE::Generated::CausalAdmission::CausalAdmission::Deliver;
        if (sce_failure_.failed()) {
            return SCE::Forge::AlgorithmResult<SCE::Generated::CausalAdmission::CausalAdmission>::failure(
                sce_failure_.error());
        }
    }
    {
        const SCE::Generated::CausalAdmission::CausalAdmission sce_value_ = result;
        if (sce_failure_.failed()) {
            return SCE::Forge::AlgorithmResult<SCE::Generated::CausalAdmission::CausalAdmission>::failure(
                sce_failure_.error());
        }
        return SCE::Forge::AlgorithmResult<SCE::Generated::CausalAdmission::CausalAdmission>::success(sce_value_);
    }
}

}  // namespace SCE::Generated::CausalAdmit

#endif  // SCE_FORGE_CAUSAL_ADMIT_H
