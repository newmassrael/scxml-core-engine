// SCE-GENERATED — DO NOT EDIT
// source-hash: 52dec3eb3cccdf24690d474eaf44f8ac67bafec1b1ed5060c73db2a6256056db
// SCE-MAP: register_values.scxml:21 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.register_values`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.register_values

import com.sce.generated.mv_value.*

fun registerValues(values: List<MvValuePayload>, id: ULong, field: ULong): com.sce.forge.runtime.AlgorithmResult<List<MvValuePayload>> {
    // SCE_FORGE.md §3.4.1: a checked operation throws its failure, and this
    // boundary hands it to the caller as a value — never as an exception.
    try {
        val out = ArrayList<MvValuePayload>(512)
        var started: Boolean = false
        var lastWall: Long = 0
        var lastCounter: UInt = 0.toUInt()
        var lastNode: ULong = 0.toULong()
        var more: Boolean = true
        while (more) {
            var chosen: Boolean = false
            var bestWall: Long = 0
            var bestCounter: UInt = 0.toUInt()
            var bestNode: ULong = 0.toULong()
            for (v in values) {
                if (v.id == id && v.field == field && (!started || v.wallTime > lastWall || v.wallTime == lastWall && (v.counter > lastCounter || v.counter == lastCounter && v.nodeId > lastNode)) && (!chosen || v.wallTime < bestWall || v.wallTime == bestWall && (v.counter < bestCounter || v.counter == bestCounter && v.nodeId < bestNode))) {
                    chosen = true
                    bestWall = v.wallTime
                    bestCounter = v.counter
                    bestNode = v.nodeId
                }
            }
            if (chosen) {
                for (w in values) {
                    if (w.id == id && w.field == field && w.wallTime == bestWall && w.counter == bestCounter && w.nodeId == bestNode) {
                        out.add(w)
                    }
                }
                started = true
                lastWall = bestWall
                lastCounter = bestCounter
                lastNode = bestNode
            } else {
                more = false
            }
        }
        return com.sce.forge.runtime.AlgorithmResult.Ok(out)
    } catch (failure: com.sce.forge.runtime.AlgorithmFailure) {
        return com.sce.forge.runtime.AlgorithmResult.Failed(failure.error)
    }
}
