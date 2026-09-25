// SCE-MAP: algorithm_checked_arith:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: free function in `namespace SCE::Generated::<Name>`. No
// STL containers, no exceptions. `bytes` lowers to `std::span<const
// std::uint8_t>` (RFC §synth-5-J-5 emitter table).

#pragma once
#ifndef SCE_FORGE_ALGORITHM_CHECKED_ARITH_H
#define SCE_FORGE_ALGORITHM_CHECKED_ARITH_H

#include <cstddef>
#include <cstdint>
#include <sce/forge/algorithm.h>
namespace SCE::Generated::AlgorithmCheckedArith {

inline SCE::Forge::AlgorithmResult<int32_t> algorithm_checked_arith(int32_t a, int32_t b, uint8_t op) {
    // SCE_FORGE.md §3.4.1: each checked operation records a failure here, and
    // the statement around it returns it before the next statement runs.
    SCE::Forge::AlgorithmFailure sce_failure_;
    int32_t r = 0;
    if (sce_failure_.failed()) return SCE::Forge::AlgorithmResult<int32_t>::failure(sce_failure_.error());
    if (const bool sce_cond1_ = op == 0; sce_failure_.failed()) {
        return SCE::Forge::AlgorithmResult<int32_t>::failure(sce_failure_.error());
    } else if (sce_cond1_) {
        r = SCE::Forge::Checked::add<std::int32_t>(sce_failure_, a, b);
        if (sce_failure_.failed()) return SCE::Forge::AlgorithmResult<int32_t>::failure(sce_failure_.error());
    }
    if (const bool sce_cond1_ = op == 1; sce_failure_.failed()) {
        return SCE::Forge::AlgorithmResult<int32_t>::failure(sce_failure_.error());
    } else if (sce_cond1_) {
        r = SCE::Forge::Checked::sub<std::int32_t>(sce_failure_, a, b);
        if (sce_failure_.failed()) return SCE::Forge::AlgorithmResult<int32_t>::failure(sce_failure_.error());
    }
    if (const bool sce_cond1_ = op == 2; sce_failure_.failed()) {
        return SCE::Forge::AlgorithmResult<int32_t>::failure(sce_failure_.error());
    } else if (sce_cond1_) {
        r = SCE::Forge::Checked::mul<std::int32_t>(sce_failure_, a, b);
        if (sce_failure_.failed()) return SCE::Forge::AlgorithmResult<int32_t>::failure(sce_failure_.error());
    }
    if (const bool sce_cond1_ = op == 3; sce_failure_.failed()) {
        return SCE::Forge::AlgorithmResult<int32_t>::failure(sce_failure_.error());
    } else if (sce_cond1_) {
        r = SCE::Forge::Checked::div<std::int32_t>(sce_failure_, a, b);
        if (sce_failure_.failed()) return SCE::Forge::AlgorithmResult<int32_t>::failure(sce_failure_.error());
    }
    if (const bool sce_cond1_ = op == 4; sce_failure_.failed()) {
        return SCE::Forge::AlgorithmResult<int32_t>::failure(sce_failure_.error());
    } else if (sce_cond1_) {
        r = SCE::Forge::Checked::rem<std::int32_t>(sce_failure_, a, b);
        if (sce_failure_.failed()) return SCE::Forge::AlgorithmResult<int32_t>::failure(sce_failure_.error());
    }
    if (const bool sce_cond1_ = op == 5; sce_failure_.failed()) {
        return SCE::Forge::AlgorithmResult<int32_t>::failure(sce_failure_.error());
    } else if (sce_cond1_) {
        r = SCE::Forge::Checked::neg<std::int32_t>(sce_failure_, a);
        if (sce_failure_.failed()) return SCE::Forge::AlgorithmResult<int32_t>::failure(sce_failure_.error());
    }
    if (const bool sce_cond1_ = op == 6; sce_failure_.failed()) {
        return SCE::Forge::AlgorithmResult<int32_t>::failure(sce_failure_.error());
    } else if (sce_cond1_) {
        r = SCE::Forge::Checked::sub<std::uint8_t>(sce_failure_, op, 7);
        if (sce_failure_.failed()) return SCE::Forge::AlgorithmResult<int32_t>::failure(sce_failure_.error());
    }
    {
        const int32_t sce_value_ = r;
        if (sce_failure_.failed()) return SCE::Forge::AlgorithmResult<int32_t>::failure(sce_failure_.error());
        return SCE::Forge::AlgorithmResult<int32_t>::success(sce_value_);
    }
}

}  // namespace SCE::Generated::AlgorithmCheckedArith

#endif  // SCE_FORGE_ALGORITHM_CHECKED_ARITH_H
