// SCE-GENERATED — DO NOT EDIT
// source-hash: 84ff81b033009528cb45ea402416f0964c32998c99c764e4cae0b42faa8f879e
// source: ordering_prune.scxml
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
