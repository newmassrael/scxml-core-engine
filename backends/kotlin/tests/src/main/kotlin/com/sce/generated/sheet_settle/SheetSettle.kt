// SCE-GENERATED — DO NOT EDIT
// source-hash: a45b1edeebf5addf07d55990d33be593d588441249c6330a80971f056631ac90
// SCE-MAP: sheet_settle.scxml:63 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.sheet_settle`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.sheet_settle

import com.sce.generated.lww_cell.*
import com.sce.generated.rga_node.*
import com.sce.generated.rga_gone.*
import com.sce.generated.sheet_value.*
import com.sce.generated.sheet_apply.*

import com.sce.forge.runtime.SceListBuf

fun sheetSettle(cells: List<LwwCellPayload>, rows: List<RgaNodePayload>, gone: List<RgaGonePayload>, cols: List<RgaNodePayload>, goneCols: List<RgaGonePayload>): com.sce.forge.runtime.AlgorithmResult<LongArray> {
    // SCE_FORGE.md §3.4.1: a checked operation throws its failure, and this
    // boundary hands it to the caller as a value — never as an exception.
    try {
        val trail = SceListBuf(768)
        var moving: Boolean = true
        var passes: Long = 0
        while (moving && passes <= (cells).size) {
            moving = false
            passes = com.sce.forge.runtime.SceChecked.add(passes, 1)
            var at: Long = 0
            for (c in cells) {
                var settled: Boolean = false
                var d: Long = 0
                while (d < (trail).size) {
                    if (trail.getLong(com.sce.forge.runtime.SceChecked.index((d).toLong(), trail.size)) == at) {
                        settled = true
                    }
                    d = com.sce.forge.runtime.SceChecked.add(d, 3)
                }
                if (!settled) {
                    var ready: Boolean = true
                    var v: Long = 0
                    var f: Long = 0
                    var va: Long = 0
                    var fa: Long = 0
                    var vb: Long = 0
                    var fb: Long = 0
                    var sides: Long = if (c.op >= 1.toUByte() && c.op <= 5.toUByte()) if (c.op == 1.toUByte()) 1 else 2 else 0
                    var side: Long = 0
                    while (side < sides) {
                        var isRef: Boolean = if (side == 0L) c.aRef else c.bRef
                        var wantRow: ULong = if (side == 0L) c.aRow else c.bRow
                        var wantCol: ULong = if (side == 0L) c.aCol else c.bCol
                        var xv: Long = if (side == 0L) c.aVal else c.bVal
                        var xf: Long = 0
                        if (isRef) {
                            xv = 0
                            var found: Boolean = false
                            var holder: Long = 0
                            var j: Long = 0
                            for (probe in cells) {
                                if (!found && probe.row == wantRow && probe.col == wantCol) {
                                    found = true
                                    holder = j
                                }
                                j = com.sce.forge.runtime.SceChecked.add(j, 1)
                            }
                            if (found) {
                                var hit: Boolean = false
                                var m: Long = 0
                                while (m < (trail).size) {
                                    if (trail.getLong(com.sce.forge.runtime.SceChecked.index((m).toLong(), trail.size)) == holder) {
                                        hit = true
                                        xv = trail.getLong(com.sce.forge.runtime.SceChecked.index((com.sce.forge.runtime.SceChecked.add(m, 1)).toLong(), trail.size))
                                        xf = trail.getLong(com.sce.forge.runtime.SceChecked.index((com.sce.forge.runtime.SceChecked.add(m, 2)).toLong(), trail.size))
                                    }
                                    m = com.sce.forge.runtime.SceChecked.add(m, 3)
                                }
                                if (!hit) {
                                    ready = false
                                }
                            }
                        }
                        if (side == 0L) {
                            va = xv
                            fa = xf
                        } else {
                            vb = xv
                            fb = xf
                        }
                        side = com.sce.forge.runtime.SceChecked.add(side, 1)
                    }
                    if (c.op == 0.toUByte()) {
                        v = c.value
                    }
                    if (c.op == 1.toUByte()) {
                        v = va
                        f = fa
                    }
                    if (c.op >= 2.toUByte() && c.op <= 5.toUByte()) {
                        if (fa != 0L || fb != 0L) {
                            f = if (fa == 0L) fb else if (fb == 0L) fa else if (fa < fb) fa else fb
                        } else {
                            var applied: SheetValuePayload = com.sce.forge.runtime.SceChecked.take(sheetApply(c.op, va, vb))
                            v = applied.value
                            f = applied.fault.toLong()
                        }
                    }
                    if (c.op > 6.toUByte()) {
                        f = 5
                    }
                    if (c.op == 6.toUByte()) {
                        var ra: Long = 0
                        var rb: Long = 0
                        var ca: Long = 0
                        var cb: Long = 0
                        var spin: Long = 0
                        for (rw in rows) {
                            spin = com.sce.forge.runtime.SceChecked.add(spin, 1)
                            if (rw.key == c.aRow) {
                                ra = spin
                            }
                            if (rw.key == c.bRow) {
                                rb = spin
                            }
                        }
                        spin = 0
                        for (cl in cols) {
                            spin = com.sce.forge.runtime.SceChecked.add(spin, 1)
                            if (cl.key == c.aCol) {
                                ca = spin
                            }
                            if (cl.key == c.bCol) {
                                cb = spin
                            }
                        }
                        if (!c.aRef || !c.bRef || ra == 0L || rb == 0L || ca == 0L || cb == 0L) {
                            f = 4
                        } else {
                            var rowLow: Long = if (ra < rb) ra else rb
                            var rowHigh: Long = if (ra < rb) rb else ra
                            var colLow: Long = if (ca < cb) ca else cb
                            var colHigh: Long = if (ca < cb) cb else ca
                            var above: Long = 0
                            var below: Long = 0
                            var spilled: Boolean = false
                            var worst: Long = 0
                            var walked: Long = 0
                            for (q in cells) {
                                var qr: Long = 0
                                var qc: Long = 0
                                var qs: Long = 0
                                for (rq in rows) {
                                    qs = com.sce.forge.runtime.SceChecked.add(qs, 1)
                                    if (rq.key == q.row) {
                                        qr = qs
                                    }
                                }
                                qs = 0
                                for (cq in cols) {
                                    qs = com.sce.forge.runtime.SceChecked.add(qs, 1)
                                    if (cq.key == q.col) {
                                        qc = qs
                                    }
                                }
                                var qgone: Boolean = false
                                for (gq in gone) {
                                    if (gq.key == q.row) {
                                        qgone = true
                                    }
                                }
                                for (hq in goneCols) {
                                    if (hq.key == q.col) {
                                        qgone = true
                                    }
                                }
                                if (!qgone && qr >= rowLow && qr <= rowHigh && qc >= colLow && qc <= colHigh) {
                                    var seen: Boolean = false
                                    var w: Long = 0
                                    while (w < (trail).size) {
                                        if (trail.getLong(com.sce.forge.runtime.SceChecked.index((w).toLong(), trail.size)) == walked) {
                                            seen = true
                                            var mv: Long = trail.getLong(com.sce.forge.runtime.SceChecked.index((com.sce.forge.runtime.SceChecked.add(w, 1)).toLong(), trail.size))
                                            var mf: Long = trail.getLong(com.sce.forge.runtime.SceChecked.index((com.sce.forge.runtime.SceChecked.add(w, 2)).toLong(), trail.size))
                                            if (mf != 0L) {
                                                worst = if (worst == 0L) mf else if (mf < worst) mf else worst
                                            }
                                            if (mf == 0L && mv > 0) {
                                                var grown: SheetValuePayload = com.sce.forge.runtime.SceChecked.take(sheetApply(2.toUByte(), above, mv))
                                                if (grown.fault != 0.toUByte()) {
                                                    spilled = true
                                                } else {
                                                    above = grown.value
                                                }
                                            }
                                            if (mf == 0L && mv < 0) {
                                                var shrunk: SheetValuePayload = com.sce.forge.runtime.SceChecked.take(sheetApply(2.toUByte(), below, mv))
                                                if (shrunk.fault != 0.toUByte()) {
                                                    spilled = true
                                                } else {
                                                    below = shrunk.value
                                                }
                                            }
                                        }
                                        w = com.sce.forge.runtime.SceChecked.add(w, 3)
                                    }
                                    if (!seen) {
                                        ready = false
                                    }
                                }
                                walked = com.sce.forge.runtime.SceChecked.add(walked, 1)
                            }
                            if (worst != 0L) {
                                f = worst
                            } else {
                                if (spilled) {
                                    f = 3
                                } else {
                                    v = com.sce.forge.runtime.SceChecked.add(above, below)
                                }
                            }
                        }
                    }
                    if (ready) {
                        trail.add(at)
                        trail.add(v)
                        trail.add(f)
                        moving = true
                    }
                }
                at = com.sce.forge.runtime.SceChecked.add(at, 1)
            }
        }
        return com.sce.forge.runtime.AlgorithmResult.Ok(trail.toLongArray())
    } catch (failure: com.sce.forge.runtime.AlgorithmFailure) {
        return com.sce.forge.runtime.AlgorithmResult.Failed(failure.error)
    }
}
