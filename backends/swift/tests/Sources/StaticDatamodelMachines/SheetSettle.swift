// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
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
    public static func sheetSettle(_ cells: [LwwCellPayload], _ rows: [RgaNodePayload], _ gone: [RgaGonePayload], _ cols: [RgaNodePayload], _ goneCols: [RgaGonePayload]) throws -> [Int64] {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        var trail: [Int64] = []
        trail.reserveCapacity(768)
        try sceFailure.throwIfFailed()
        var moving: Bool = true
        try sceFailure.throwIfFailed()
        var passes: Int64 = 0
        try sceFailure.throwIfFailed()
        while true {
            let sceCond2 = moving && passes <= Int64(truncatingIfNeeded: cells.count)
            try sceFailure.throwIfFailed()
            if !sceCond2 { break }
            moving = false
            try sceFailure.throwIfFailed()
            passes = Checked.add(sceFailure, passes, 1)
            try sceFailure.throwIfFailed()
            var at: Int64 = 0
            try sceFailure.throwIfFailed()
            for c in cells {
                var settled: Bool = false
                try sceFailure.throwIfFailed()
                var d: Int64 = 0
                try sceFailure.throwIfFailed()
                while true {
                    let sceCond4 = d < Int64(truncatingIfNeeded: trail.count)
                    try sceFailure.throwIfFailed()
                    if !sceCond4 { break }
                    do {
                        let sceCond5 = Checked.at(sceFailure, trail, d, fallback: 0) == at
                        try sceFailure.throwIfFailed()
                        if sceCond5 {
                            settled = true
                            try sceFailure.throwIfFailed()
                        }
                    }
                    d = Checked.add(sceFailure, d, 3)
                    try sceFailure.throwIfFailed()
                }
                do {
                    let sceCond4 = !settled
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        var ready: Bool = true
                        try sceFailure.throwIfFailed()
                        var v: Int64 = 0
                        try sceFailure.throwIfFailed()
                        var f: Int64 = 0
                        try sceFailure.throwIfFailed()
                        var va: Int64 = 0
                        try sceFailure.throwIfFailed()
                        var fa: Int64 = 0
                        try sceFailure.throwIfFailed()
                        var vb: Int64 = 0
                        try sceFailure.throwIfFailed()
                        var fb: Int64 = 0
                        try sceFailure.throwIfFailed()
                        let sides: Int64 = c.op >= 1 && c.op <= 5 ? (c.op == 1 ? 1 : 2) : 0
                        try sceFailure.throwIfFailed()
                        var side: Int64 = 0
                        try sceFailure.throwIfFailed()
                        while true {
                            let sceCond6 = side < sides
                            try sceFailure.throwIfFailed()
                            if !sceCond6 { break }
                            let isRef: Bool = side == 0 ? c.aRef : c.bRef
                            try sceFailure.throwIfFailed()
                            let wantRow: UInt64 = side == 0 ? c.aRow : c.bRow
                            try sceFailure.throwIfFailed()
                            let wantCol: UInt64 = side == 0 ? c.aCol : c.bCol
                            try sceFailure.throwIfFailed()
                            var xv: Int64 = side == 0 ? c.aVal : c.bVal
                            try sceFailure.throwIfFailed()
                            var xf: Int64 = 0
                            try sceFailure.throwIfFailed()
                            do {
                                let sceCond7 = isRef
                                try sceFailure.throwIfFailed()
                                if sceCond7 {
                                    xv = 0
                                    try sceFailure.throwIfFailed()
                                    var found: Bool = false
                                    try sceFailure.throwIfFailed()
                                    var holder: Int64 = 0
                                    try sceFailure.throwIfFailed()
                                    var j: Int64 = 0
                                    try sceFailure.throwIfFailed()
                                    for probe in cells {
                                        do {
                                            let sceCond10 = !found && probe.row == wantRow && probe.col == wantCol
                                            try sceFailure.throwIfFailed()
                                            if sceCond10 {
                                                found = true
                                                try sceFailure.throwIfFailed()
                                                holder = j
                                                try sceFailure.throwIfFailed()
                                            }
                                        }
                                        j = Checked.add(sceFailure, j, 1)
                                        try sceFailure.throwIfFailed()
                                    }
                                    do {
                                        let sceCond9 = found
                                        try sceFailure.throwIfFailed()
                                        if sceCond9 {
                                            var hit: Bool = false
                                            try sceFailure.throwIfFailed()
                                            var m: Int64 = 0
                                            try sceFailure.throwIfFailed()
                                            while true {
                                                let sceCond11 = m < Int64(truncatingIfNeeded: trail.count)
                                                try sceFailure.throwIfFailed()
                                                if !sceCond11 { break }
                                                do {
                                                    let sceCond12 = Checked.at(sceFailure, trail, m, fallback: 0) == holder
                                                    try sceFailure.throwIfFailed()
                                                    if sceCond12 {
                                                        hit = true
                                                        try sceFailure.throwIfFailed()
                                                        xv = Checked.at(sceFailure, trail, Checked.add(sceFailure, m, 1), fallback: 0)
                                                        try sceFailure.throwIfFailed()
                                                        xf = Checked.at(sceFailure, trail, Checked.add(sceFailure, m, 2), fallback: 0)
                                                        try sceFailure.throwIfFailed()
                                                    }
                                                }
                                                m = Checked.add(sceFailure, m, 3)
                                                try sceFailure.throwIfFailed()
                                            }
                                            do {
                                                let sceCond11 = !hit
                                                try sceFailure.throwIfFailed()
                                                if sceCond11 {
                                                    ready = false
                                                    try sceFailure.throwIfFailed()
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            do {
                                let sceCond7 = side == 0
                                try sceFailure.throwIfFailed()
                                if sceCond7 {
                                    va = xv
                                    try sceFailure.throwIfFailed()
                                    fa = xf
                                    try sceFailure.throwIfFailed()
                                } else {
                                    vb = xv
                                    try sceFailure.throwIfFailed()
                                    fb = xf
                                    try sceFailure.throwIfFailed()
                                }
                            }
                            side = Checked.add(sceFailure, side, 1)
                            try sceFailure.throwIfFailed()
                        }
                        do {
                            let sceCond6 = c.op == 0
                            try sceFailure.throwIfFailed()
                            if sceCond6 {
                                v = c.value
                                try sceFailure.throwIfFailed()
                            }
                        }
                        do {
                            let sceCond6 = c.op == 1
                            try sceFailure.throwIfFailed()
                            if sceCond6 {
                                v = va
                                try sceFailure.throwIfFailed()
                                f = fa
                                try sceFailure.throwIfFailed()
                            }
                        }
                        do {
                            let sceCond6 = c.op >= 2 && c.op <= 5
                            try sceFailure.throwIfFailed()
                            if sceCond6 {
                                do {
                                    let sceCond8 = fa != 0 || fb != 0
                                    try sceFailure.throwIfFailed()
                                    if sceCond8 {
                                        f = fa == 0 ? fb : (fb == 0 ? fa : (fa < fb ? fa : fb))
                                        try sceFailure.throwIfFailed()
                                    } else {
                                        let applied: SheetValuePayload = Checked.takeDefault(sceFailure, { try SheetApply.sheetApply(c.op, va, vb) })
                                        try sceFailure.throwIfFailed()
                                        v = applied.value
                                        try sceFailure.throwIfFailed()
                                        f = Int64(applied.fault)
                                        try sceFailure.throwIfFailed()
                                    }
                                }
                            }
                        }
                        do {
                            let sceCond6 = c.op > 6
                            try sceFailure.throwIfFailed()
                            if sceCond6 {
                                f = 5
                                try sceFailure.throwIfFailed()
                            }
                        }
                        do {
                            let sceCond6 = c.op == 6
                            try sceFailure.throwIfFailed()
                            if sceCond6 {
                                var ra: Int64 = 0
                                try sceFailure.throwIfFailed()
                                var rb: Int64 = 0
                                try sceFailure.throwIfFailed()
                                var ca: Int64 = 0
                                try sceFailure.throwIfFailed()
                                var cb: Int64 = 0
                                try sceFailure.throwIfFailed()
                                var spin: Int64 = 0
                                try sceFailure.throwIfFailed()
                                for rw in rows {
                                    spin = Checked.add(sceFailure, spin, 1)
                                    try sceFailure.throwIfFailed()
                                    do {
                                        let sceCond9 = rw.key == c.aRow
                                        try sceFailure.throwIfFailed()
                                        if sceCond9 {
                                            ra = spin
                                            try sceFailure.throwIfFailed()
                                        }
                                    }
                                    do {
                                        let sceCond9 = rw.key == c.bRow
                                        try sceFailure.throwIfFailed()
                                        if sceCond9 {
                                            rb = spin
                                            try sceFailure.throwIfFailed()
                                        }
                                    }
                                }
                                spin = 0
                                try sceFailure.throwIfFailed()
                                for cl in cols {
                                    spin = Checked.add(sceFailure, spin, 1)
                                    try sceFailure.throwIfFailed()
                                    do {
                                        let sceCond9 = cl.key == c.aCol
                                        try sceFailure.throwIfFailed()
                                        if sceCond9 {
                                            ca = spin
                                            try sceFailure.throwIfFailed()
                                        }
                                    }
                                    do {
                                        let sceCond9 = cl.key == c.bCol
                                        try sceFailure.throwIfFailed()
                                        if sceCond9 {
                                            cb = spin
                                            try sceFailure.throwIfFailed()
                                        }
                                    }
                                }
                                do {
                                    let sceCond8 = !c.aRef || !c.bRef || ra == 0 || rb == 0 || ca == 0 || cb == 0
                                    try sceFailure.throwIfFailed()
                                    if sceCond8 {
                                        f = 4
                                        try sceFailure.throwIfFailed()
                                    } else {
                                        let rowLow: Int64 = ra < rb ? ra : rb
                                        try sceFailure.throwIfFailed()
                                        let rowHigh: Int64 = ra < rb ? rb : ra
                                        try sceFailure.throwIfFailed()
                                        let colLow: Int64 = ca < cb ? ca : cb
                                        try sceFailure.throwIfFailed()
                                        let colHigh: Int64 = ca < cb ? cb : ca
                                        try sceFailure.throwIfFailed()
                                        var above: Int64 = 0
                                        try sceFailure.throwIfFailed()
                                        var below: Int64 = 0
                                        try sceFailure.throwIfFailed()
                                        var spilled: Bool = false
                                        try sceFailure.throwIfFailed()
                                        var worst: Int64 = 0
                                        try sceFailure.throwIfFailed()
                                        var walked: Int64 = 0
                                        try sceFailure.throwIfFailed()
                                        for q in cells {
                                            var qr: Int64 = 0
                                            try sceFailure.throwIfFailed()
                                            var qc: Int64 = 0
                                            try sceFailure.throwIfFailed()
                                            var qs: Int64 = 0
                                            try sceFailure.throwIfFailed()
                                            for rq in rows {
                                                qs = Checked.add(sceFailure, qs, 1)
                                                try sceFailure.throwIfFailed()
                                                do {
                                                    let sceCond12 = rq.key == q.row
                                                    try sceFailure.throwIfFailed()
                                                    if sceCond12 {
                                                        qr = qs
                                                        try sceFailure.throwIfFailed()
                                                    }
                                                }
                                            }
                                            qs = 0
                                            try sceFailure.throwIfFailed()
                                            for cq in cols {
                                                qs = Checked.add(sceFailure, qs, 1)
                                                try sceFailure.throwIfFailed()
                                                do {
                                                    let sceCond12 = cq.key == q.col
                                                    try sceFailure.throwIfFailed()
                                                    if sceCond12 {
                                                        qc = qs
                                                        try sceFailure.throwIfFailed()
                                                    }
                                                }
                                            }
                                            var qgone: Bool = false
                                            try sceFailure.throwIfFailed()
                                            for gq in gone {
                                                do {
                                                    let sceCond12 = gq.key == q.row
                                                    try sceFailure.throwIfFailed()
                                                    if sceCond12 {
                                                        qgone = true
                                                        try sceFailure.throwIfFailed()
                                                    }
                                                }
                                            }
                                            for hq in goneCols {
                                                do {
                                                    let sceCond12 = hq.key == q.col
                                                    try sceFailure.throwIfFailed()
                                                    if sceCond12 {
                                                        qgone = true
                                                        try sceFailure.throwIfFailed()
                                                    }
                                                }
                                            }
                                            do {
                                                let sceCond11 = !qgone && qr >= rowLow && qr <= rowHigh && qc >= colLow && qc <= colHigh
                                                try sceFailure.throwIfFailed()
                                                if sceCond11 {
                                                    var seen: Bool = false
                                                    try sceFailure.throwIfFailed()
                                                    var w: Int64 = 0
                                                    try sceFailure.throwIfFailed()
                                                    while true {
                                                        let sceCond13 = w < Int64(truncatingIfNeeded: trail.count)
                                                        try sceFailure.throwIfFailed()
                                                        if !sceCond13 { break }
                                                        do {
                                                            let sceCond14 = Checked.at(sceFailure, trail, w, fallback: 0) == walked
                                                            try sceFailure.throwIfFailed()
                                                            if sceCond14 {
                                                                seen = true
                                                                try sceFailure.throwIfFailed()
                                                                let mv: Int64 = Checked.at(sceFailure, trail, Checked.add(sceFailure, w, 1), fallback: 0)
                                                                try sceFailure.throwIfFailed()
                                                                let mf: Int64 = Checked.at(sceFailure, trail, Checked.add(sceFailure, w, 2), fallback: 0)
                                                                try sceFailure.throwIfFailed()
                                                                do {
                                                                    let sceCond16 = mf != 0
                                                                    try sceFailure.throwIfFailed()
                                                                    if sceCond16 {
                                                                        worst = worst == 0 ? mf : (mf < worst ? mf : worst)
                                                                        try sceFailure.throwIfFailed()
                                                                    }
                                                                }
                                                                do {
                                                                    let sceCond16 = mf == 0 && mv > 0
                                                                    try sceFailure.throwIfFailed()
                                                                    if sceCond16 {
                                                                        let grown: SheetValuePayload = Checked.takeDefault(sceFailure, { try SheetApply.sheetApply(2, above, mv) })
                                                                        try sceFailure.throwIfFailed()
                                                                        do {
                                                                            let sceCond18 = grown.fault != 0
                                                                            try sceFailure.throwIfFailed()
                                                                            if sceCond18 {
                                                                                spilled = true
                                                                                try sceFailure.throwIfFailed()
                                                                            } else {
                                                                                above = grown.value
                                                                                try sceFailure.throwIfFailed()
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                do {
                                                                    let sceCond16 = mf == 0 && mv < 0
                                                                    try sceFailure.throwIfFailed()
                                                                    if sceCond16 {
                                                                        let shrunk: SheetValuePayload = Checked.takeDefault(sceFailure, { try SheetApply.sheetApply(2, below, mv) })
                                                                        try sceFailure.throwIfFailed()
                                                                        do {
                                                                            let sceCond18 = shrunk.fault != 0
                                                                            try sceFailure.throwIfFailed()
                                                                            if sceCond18 {
                                                                                spilled = true
                                                                                try sceFailure.throwIfFailed()
                                                                            } else {
                                                                                below = shrunk.value
                                                                                try sceFailure.throwIfFailed()
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        w = Checked.add(sceFailure, w, 3)
                                                        try sceFailure.throwIfFailed()
                                                    }
                                                    do {
                                                        let sceCond13 = !seen
                                                        try sceFailure.throwIfFailed()
                                                        if sceCond13 {
                                                            ready = false
                                                            try sceFailure.throwIfFailed()
                                                        }
                                                    }
                                                }
                                            }
                                            walked = Checked.add(sceFailure, walked, 1)
                                            try sceFailure.throwIfFailed()
                                        }
                                        do {
                                            let sceCond10 = worst != 0
                                            try sceFailure.throwIfFailed()
                                            if sceCond10 {
                                                f = worst
                                                try sceFailure.throwIfFailed()
                                            } else {
                                                do {
                                                    let sceCond12 = spilled
                                                    try sceFailure.throwIfFailed()
                                                    if sceCond12 {
                                                        f = 3
                                                        try sceFailure.throwIfFailed()
                                                    } else {
                                                        v = Checked.add(sceFailure, above, below)
                                                        try sceFailure.throwIfFailed()
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        do {
                            let sceCond6 = ready
                            try sceFailure.throwIfFailed()
                            if sceCond6 {
                                trail.append(at)
                                try sceFailure.throwIfFailed()
                                trail.append(v)
                                try sceFailure.throwIfFailed()
                                trail.append(f)
                                try sceFailure.throwIfFailed()
                                moving = true
                                try sceFailure.throwIfFailed()
                            }
                        }
                    }
                }
                at = Checked.add(sceFailure, at, 1)
                try sceFailure.throwIfFailed()
            }
        }
        do {
            let sceReturned: [Int64] = trail
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
