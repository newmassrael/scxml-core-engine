// SCE-GENERATED — DO NOT EDIT
// source-hash: c431fa9e45990e8b729e69a4bf81ba13f9cc4e8066ddecbd5ff6346ac56dd5f3
// SCE-MAP: lww_write.scxml:24 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.lww_write`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.lww_write

import com.sce.generated.lww_cell.*
import com.sce.generated.hlc_timestamp.*
import com.sce.generated.hlc_compare.*

fun lwwWrite(cells: List<LwwCellPayload>, write: LwwCellPayload): com.sce.forge.runtime.AlgorithmResult<List<LwwCellPayload>> {
    // SCE_FORGE.md §3.4.1: a checked operation throws its failure, and this
    // boundary hands it to the caller as a value — never as an exception.
    try {
        val out = ArrayList<LwwCellPayload>(256)
        var held: Boolean = false
        var incoming = HlcTimestampPayload(wallTime = write.wallTime, counter = write.counter, nodeId = write.nodeId)
        for (c in cells) {
            if (c.row == write.row && c.col == write.col) {
                held = true
                var stored = HlcTimestampPayload(wallTime = c.wallTime, counter = c.counter, nodeId = c.nodeId)
                if (hlcCompare(incoming, stored) > 0) {
                    out.add(write)
                } else {
                    out.add(c)
                }
            } else {
                out.add(c)
            }
        }
        if (!held) {
            out.add(write)
        }
        return com.sce.forge.runtime.AlgorithmResult.Ok(out)
    } catch (failure: com.sce.forge.runtime.AlgorithmFailure) {
        return com.sce.forge.runtime.AlgorithmResult.Failed(failure.error)
    }
}
