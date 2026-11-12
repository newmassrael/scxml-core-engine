// SCE-GENERATED — DO NOT EDIT
// source-hash: fb4f42a34e7ac40a28d8e5daee5d633c3c45dddac513816edf2bb83fdc85f91c
// SCE-MAP: sheet_settle.scxml:63 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `SheetSettle`, a file of the
// module that calls it: a call is qualified by the enum
// (`SheetSettle.sheetSettle(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum SheetSettle {
    public static func sheetSettle(_ cells: [LwwCellPayload], _ rows: [RgaNodePayload], _ gone: [RgaGonePayload], _ cols: [RgaNodePayload], _ goneCols: [RgaGonePayload]) throws(AlgorithmError) -> [Int64] {
        var trail: [Int64] = []
        trail.reserveCapacity(768)
        var moving: Bool = true
        var passes: Int64 = 0
        while moving && passes <= Int64(truncatingIfNeeded: cells.count) {
            moving = false
            passes = (try Checked.add(passes, 1))
            var at: Int64 = 0
            for c in cells {
                var settled: Bool = false
                var d: Int64 = 0
                while d < Int64(truncatingIfNeeded: trail.count) {
                    if (try Checked.at(trail, d)) == at {
                        settled = true
                    }
                    d = (try Checked.add(d, 3))
                }
                if !settled {
                    var ready: Bool = true
                    var v: Int64 = 0
                    var f: Int64 = 0
                    var va: Int64 = 0
                    var fa: Int64 = 0
                    var vb: Int64 = 0
                    var fb: Int64 = 0
                    let sides: Int64 = c.op >= 1 && c.op <= 5 ? (c.op == 1 ? 1 : 2) : 0
                    var side: Int64 = 0
                    while side < sides {
                        let isRef: Bool = side == 0 ? c.aRef : c.bRef
                        let wantRow: UInt64 = side == 0 ? c.aRow : c.bRow
                        let wantCol: UInt64 = side == 0 ? c.aCol : c.bCol
                        var xv: Int64 = side == 0 ? c.aVal : c.bVal
                        var xf: Int64 = 0
                        if isRef {
                            xv = 0
                            var found: Bool = false
                            var holder: Int64 = 0
                            var j: Int64 = 0
                            for probe in cells {
                                if !found && probe.row == wantRow && probe.col == wantCol {
                                    found = true
                                    holder = j
                                }
                                j = (try Checked.add(j, 1))
                            }
                            if found {
                                var hit: Bool = false
                                var m: Int64 = 0
                                while m < Int64(truncatingIfNeeded: trail.count) {
                                    if (try Checked.at(trail, m)) == holder {
                                        hit = true
                                        xv = (try Checked.at(trail, (try Checked.add(m, 1))))
                                        xf = (try Checked.at(trail, (try Checked.add(m, 2))))
                                    }
                                    m = (try Checked.add(m, 3))
                                }
                                if !hit {
                                    ready = false
                                }
                            }
                        }
                        if side == 0 {
                            va = xv
                            fa = xf
                        } else {
                            vb = xv
                            fb = xf
                        }
                        side = (try Checked.add(side, 1))
                    }
                    if c.op == 0 {
                        v = c.value
                    }
                    if c.op == 1 {
                        v = va
                        f = fa
                    }
                    if c.op >= 2 && c.op <= 5 {
                        if fa != 0 || fb != 0 {
                            f = fa == 0 ? fb : (fb == 0 ? fa : (fa < fb ? fa : fb))
                        } else {
                            let applied: SheetValuePayload = (try SheetApply.sheetApply(c.op, va, vb))
                            v = applied.value
                            f = Int64(applied.fault)
                        }
                    }
                    if c.op > 6 {
                        f = 5
                    }
                    if c.op == 6 {
                        var ra: Int64 = 0
                        var rb: Int64 = 0
                        var ca: Int64 = 0
                        var cb: Int64 = 0
                        var spin: Int64 = 0
                        for rw in rows {
                            spin = (try Checked.add(spin, 1))
                            if rw.key == c.aRow {
                                ra = spin
                            }
                            if rw.key == c.bRow {
                                rb = spin
                            }
                        }
                        spin = 0
                        for cl in cols {
                            spin = (try Checked.add(spin, 1))
                            if cl.key == c.aCol {
                                ca = spin
                            }
                            if cl.key == c.bCol {
                                cb = spin
                            }
                        }
                        if !c.aRef || !c.bRef || ra == 0 || rb == 0 || ca == 0 || cb == 0 {
                            f = 4
                        } else {
                            let rowLow: Int64 = ra < rb ? ra : rb
                            let rowHigh: Int64 = ra < rb ? rb : ra
                            let colLow: Int64 = ca < cb ? ca : cb
                            let colHigh: Int64 = ca < cb ? cb : ca
                            var above: Int64 = 0
                            var below: Int64 = 0
                            var spilled: Bool = false
                            var worst: Int64 = 0
                            var walked: Int64 = 0
                            for q in cells {
                                var qr: Int64 = 0
                                var qc: Int64 = 0
                                var qs: Int64 = 0
                                for rq in rows {
                                    qs = (try Checked.add(qs, 1))
                                    if rq.key == q.row {
                                        qr = qs
                                    }
                                }
                                qs = 0
                                for cq in cols {
                                    qs = (try Checked.add(qs, 1))
                                    if cq.key == q.col {
                                        qc = qs
                                    }
                                }
                                var qgone: Bool = false
                                for gq in gone {
                                    if gq.key == q.row {
                                        qgone = true
                                    }
                                }
                                for hq in goneCols {
                                    if hq.key == q.col {
                                        qgone = true
                                    }
                                }
                                if !qgone && qr >= rowLow && qr <= rowHigh && qc >= colLow && qc <= colHigh {
                                    var seen: Bool = false
                                    var w: Int64 = 0
                                    while w < Int64(truncatingIfNeeded: trail.count) {
                                        if (try Checked.at(trail, w)) == walked {
                                            seen = true
                                            let mv: Int64 = (try Checked.at(trail, (try Checked.add(w, 1))))
                                            let mf: Int64 = (try Checked.at(trail, (try Checked.add(w, 2))))
                                            if mf != 0 {
                                                worst = worst == 0 ? mf : (mf < worst ? mf : worst)
                                            }
                                            if mf == 0 && mv > 0 {
                                                let grown: SheetValuePayload = (try SheetApply.sheetApply(2, above, mv))
                                                if grown.fault != 0 {
                                                    spilled = true
                                                } else {
                                                    above = grown.value
                                                }
                                            }
                                            if mf == 0 && mv < 0 {
                                                let shrunk: SheetValuePayload = (try SheetApply.sheetApply(2, below, mv))
                                                if shrunk.fault != 0 {
                                                    spilled = true
                                                } else {
                                                    below = shrunk.value
                                                }
                                            }
                                        }
                                        w = (try Checked.add(w, 3))
                                    }
                                    if !seen {
                                        ready = false
                                    }
                                }
                                walked = (try Checked.add(walked, 1))
                            }
                            if worst != 0 {
                                f = worst
                            } else {
                                if spilled {
                                    f = 3
                                } else {
                                    v = (try Checked.add(above, below))
                                }
                            }
                        }
                    }
                    if ready {
                        trail.append(at)
                        trail.append(v)
                        trail.append(f)
                        moving = true
                    }
                }
                at = (try Checked.add(at, 1))
            }
        }
        return trail
    }
}
