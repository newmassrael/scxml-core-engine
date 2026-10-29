// SCE-GENERATED — DO NOT EDIT
// source-hash: 75c0c87af90d82dec15f516a721dd486d47e1280589aefe06f15efdef5f4f850
// SCE-MAP: event_latest.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.event_latest`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.event_latest

import com.sce.generated.mv_value.*
import com.sce.generated.event_later.*

fun eventLatest(values: List<MvValuePayload>, id: ULong, field: ULong, none: Long): Long {
    var found: Boolean = false
    var bestWall: Long = 0
    var bestCounter: UInt = 0.toUInt()
    var bestNode: ULong = 0.toULong()
    var best: Long = none
    for (v in values) {
        if (v.id == id && v.field == field && (!found || eventLater(v.wallTime, v.counter, v.nodeId, bestWall, bestCounter, bestNode))) {
            found = true
            bestWall = v.wallTime
            bestCounter = v.counter
            bestNode = v.nodeId
            best = v.value
        }
    }
    return best
}
