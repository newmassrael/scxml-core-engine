// SCE-GENERATED — DO NOT EDIT
// source-hash: 9fab540da93829eaa9159cbc7132fde0b98312062867e02e6e631d43aef759bc
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
