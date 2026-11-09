// SCE-GENERATED — DO NOT EDIT
// source-hash: f2f8682669670c7aaedbccb3102dd71d7b187eac29a32d3d20509a5bc070c12e
// SCE-MAP: causal_admit.scxml:25 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.causal_admit`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.causal_admit

import com.sce.generated.causal_entry.*
import com.sce.generated.causal_admission.*

fun causalAdmit(delivered: List<CausalEntryPayload>, dot: CausalEntryPayload, context: List<CausalEntryPayload>): com.sce.forge.runtime.AlgorithmResult<CausalAdmission> {
    // SCE_FORGE.md §3.4.1: a checked operation throws its failure, and this
    // boundary hands it to the caller as a value — never as an exception.
    try {
        var seen: ULong = 0.toULong()
        for (mine in delivered) {
            if (mine.hi == dot.hi && mine.lo == dot.lo) {
                seen = mine.seq
            }
        }
        var covered: Boolean = true
        var have: ULong = 0.toULong()
        for (needed in context) {
            have = 0.toULong()
            for (held in delivered) {
                if (held.hi == needed.hi && held.lo == needed.lo) {
                    have = held.seq
                }
            }
            if ((needed.hi != dot.hi || needed.lo != dot.lo) && needed.seq > have) {
                covered = false
            }
        }
        var result: CausalAdmission = CausalAdmission.HOLD
        if (dot.seq <= seen) {
            result = CausalAdmission.DROP
        }
        if (dot.seq == com.sce.forge.runtime.SceChecked.add(seen, 1.toULong()) && covered) {
            result = CausalAdmission.DELIVER
        }
        return com.sce.forge.runtime.AlgorithmResult.Ok(result)
    } catch (failure: com.sce.forge.runtime.AlgorithmFailure) {
        return com.sce.forge.runtime.AlgorithmResult.Failed(failure.error)
    }
}
