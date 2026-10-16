// SCE-GENERATED — DO NOT EDIT
// source-hash: 8149e47f56da0bbb92d7b1c4600956bbfd585c43526372f3618b2e600843296d
// SCE-MAP: sheet_show.scxml:24 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.sheet_show`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.sheet_show

import com.sce.generated.lww_cell.*
import com.sce.generated.sheet_shown.*

fun sheetShow(cells: List<LwwCellPayload>, trail: LongArray): com.sce.forge.runtime.AlgorithmResult<List<SheetShownPayload>> {
    // SCE_FORGE.md §3.4.1: a checked operation throws its failure, and this
    // boundary hands it to the caller as a value — never as an exception.
    try {
        val out = ArrayList<SheetShownPayload>(256)
        var started: Boolean = false
        var lastRow: ULong = 0.toULong()
        var lastCol: ULong = 0.toULong()
        var more: Boolean = true
        while (more) {
            var chosen: Boolean = false
            var bestRow: ULong = 0.toULong()
            var bestCol: ULong = 0.toULong()
            var bestAt: Long = 0
            var cursor: Long = 0
            for (s in cells) {
                if ((!started || s.row > lastRow || s.row == lastRow && s.col > lastCol) && (!chosen || s.row < bestRow || s.row == bestRow && s.col < bestCol)) {
                    chosen = true
                    bestRow = s.row
                    bestCol = s.col
                    bestAt = cursor
                }
                cursor = com.sce.forge.runtime.SceChecked.add(cursor, 1)
            }
            if (chosen) {
                var shownValue: Long = 0
                var shownFault: UByte = 1.toUByte()
                var u: Long = 0
                while (u < (trail).size) {
                    if (com.sce.forge.runtime.SceChecked.at(trail, (u).toLong()) == bestAt) {
                        shownValue = com.sce.forge.runtime.SceChecked.at(trail, (com.sce.forge.runtime.SceChecked.add(u, 1)).toLong())
                        shownFault = com.sce.forge.runtime.SceChecked.narrowToUByte(com.sce.forge.runtime.SceChecked.at(trail, (com.sce.forge.runtime.SceChecked.add(u, 2)).toLong()))
                    }
                    u = com.sce.forge.runtime.SceChecked.add(u, 3)
                }
                var line = SheetShownPayload(row = bestRow, col = bestCol, value = shownValue, fault = shownFault)
                out.add(line)
                started = true
                lastRow = bestRow
                lastCol = bestCol
            } else {
                more = false
            }
        }
        return com.sce.forge.runtime.AlgorithmResult.Ok(out)
    } catch (failure: com.sce.forge.runtime.AlgorithmFailure) {
        return com.sce.forge.runtime.AlgorithmResult.Failed(failure.error)
    }
}
