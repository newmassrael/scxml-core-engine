// SCE-GENERATED — DO NOT EDIT
// source-hash: a45b1edeebf5addf07d55990d33be593d588441249c6330a80971f056631ac90
// SCE-MAP: mv_value_holds.scxml:13 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.mv_value_holds`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.mv_value_holds

import com.sce.generated.mv_value.*

fun mvValueHolds(values: List<MvValuePayload>, probe: MvValuePayload): Boolean {
    var found: Boolean = false
    for (v in values) {
        if (v.id == probe.id && v.field == probe.field && v.wallTime == probe.wallTime && v.counter == probe.counter && v.nodeId == probe.nodeId) {
            found = true
        }
    }
    return found
}
