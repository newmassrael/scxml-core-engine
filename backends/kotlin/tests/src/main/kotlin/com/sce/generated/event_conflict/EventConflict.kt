// SCE-GENERATED — DO NOT EDIT
// source-hash: 5d22e54cdaa26fa7d37f9212c73eac0589e479e7d084e83b6ac57dae4eb27663
// SCE-MAP: event_conflict.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.event_conflict`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.event_conflict

import com.sce.generated.mv_value.*

fun eventConflict(values: List<MvValuePayload>, id: ULong): Boolean {
    var twice: Boolean = false
    for (a in values) {
        if (a.id == id && (a.field == 9.toULong() || a.field >= 15.toULong() && a.field <= 58.toULong())) {
            for (b in values) {
                if (b.id == id && b.field == a.field && (b.wallTime != a.wallTime || b.counter != a.counter || b.nodeId != a.nodeId)) {
                    twice = true
                }
            }
        }
    }
    return twice
}
