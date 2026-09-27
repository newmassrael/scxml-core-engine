// SCE-GENERATED — DO NOT EDIT
// source-hash: ab119d19c373fb9e83e74bd30f74186a9e2f87ef70ba045b5c5ab8bb9e9d1849
// SCE-MAP: ordering_hold.scxml:16 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.ordering_hold`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.ordering_hold

import com.sce.generated.ordering_slot.*

fun orderingHold(pending: List<OrderingSlotPayload>, seq: ULong, now: Long): List<OrderingSlotPayload> {
    var arrived = OrderingSlotPayload(seq = seq, arrivedAtMs = now)
    val out = ArrayList<OrderingSlotPayload>(256)
    var placed: Boolean = false
    for (slot in pending) {
        if (slot.seq == seq) {
            placed = true
        }
        if (!placed && seq < slot.seq) {
            out.add(arrived)
            placed = true
        }
        out.add(slot)
    }
    if (!placed) {
        out.add(arrived)
    }
    return out
}
