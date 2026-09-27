// SCE-GENERATED — DO NOT EDIT
// source-hash: 418438a744050ac9cd6cb0691018fdb0ef433f86cf92fa1a16f148307512f21e
// SCE-MAP: ordering_gap_end.scxml:21 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.ordering_gap_end`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.ordering_gap_end

import com.sce.generated.ordering_slot.*

fun orderingGapEnd(pending: List<OrderingSlotPayload>, next: ULong, now: Long, timeoutMs: Long): com.sce.forge.runtime.AlgorithmResult<ULong> {
    // SCE_FORGE.md §3.4.1: a checked operation throws its failure, and this
    // boundary hands it to the caller as a value — never as an exception.
    try {
        var resume: ULong = next
        var first: Boolean = true
        for (slot in pending) {
            if (first && slot.seq != next && com.sce.forge.runtime.SceChecked.sub(now, slot.arrivedAtMs) >= timeoutMs) {
                resume = slot.seq
            }
            first = false
        }
        return com.sce.forge.runtime.AlgorithmResult.Ok(resume)
    } catch (failure: com.sce.forge.runtime.AlgorithmFailure) {
        return com.sce.forge.runtime.AlgorithmResult.Failed(failure.error)
    }
}
