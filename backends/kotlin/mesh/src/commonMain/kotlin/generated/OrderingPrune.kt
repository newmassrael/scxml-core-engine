// SCE-GENERATED — DO NOT EDIT
// source-hash: f3593d0da7c47c6e95770dd740ec6b7f10d9172b65629c75948b0c0e547691b7
// SCE-MAP: ordering_prune.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.ordering_prune`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.ordering_prune

import com.sce.generated.ordering_slot.*

fun orderingPrune(pending: List<OrderingSlotPayload>, next: ULong): List<OrderingSlotPayload> {
    val out = ArrayList<OrderingSlotPayload>(256)
    for (slot in pending) {
        if (slot.seq >= next) {
            out.add(slot)
        }
    }
    return out
}
