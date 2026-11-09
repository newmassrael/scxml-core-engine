// SCE-GENERATED — DO NOT EDIT
// source-hash: f2f8682669670c7aaedbccb3102dd71d7b187eac29a32d3d20509a5bc070c12e
// SCE-MAP: causal_raise.scxml:21 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.causal_raise`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.causal_raise

import com.sce.generated.causal_entry.*

fun causalRaise(vector: List<CausalEntryPayload>, other: List<CausalEntryPayload>): List<CausalEntryPayload> {
    val out = ArrayList<CausalEntryPayload>(64)
    var top: ULong = 0.toULong()
    for (mine in vector) {
        top = mine.seq
        for (theirs in other) {
            if (theirs.hi == mine.hi && theirs.lo == mine.lo && theirs.seq > top) {
                top = theirs.seq
            }
        }
        var raised = CausalEntryPayload(hi = mine.hi, lo = mine.lo, seq = top)
        out.add(raised)
    }
    var named: Boolean = false
    for (added in other) {
        named = false
        for (known in vector) {
            if (known.hi == added.hi && known.lo == added.lo) {
                named = true
            }
        }
        if (!named) {
            out.add(added)
        }
    }
    return out
}
