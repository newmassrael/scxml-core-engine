// SCE-GENERATED — DO NOT EDIT
// source-hash: fb4f42a34e7ac40a28d8e5daee5d633c3c45dddac513816edf2bb83fdc85f91c
// SCE-MAP: sheet_show.scxml:24 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `SheetShow`, a file of the
// module that calls it: a call is qualified by the enum
// (`SheetShow.sheetShow(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum SheetShow {
    public static func sheetShow(_ cells: [LwwCellPayload], _ trail: [Int64]) throws(AlgorithmError) -> [SheetShownPayload] {
        var out: [SheetShownPayload] = []
        out.reserveCapacity(256)
        var started: Bool = false
        var lastRow: UInt64 = 0
        var lastCol: UInt64 = 0
        var more: Bool = true
        while more {
            var chosen: Bool = false
            var bestRow: UInt64 = 0
            var bestCol: UInt64 = 0
            var bestAt: Int64 = 0
            var cursor: Int64 = 0
            for s in cells {
                if (!started || s.row > lastRow || s.row == lastRow && s.col > lastCol) && (!chosen || s.row < bestRow || s.row == bestRow && s.col < bestCol) {
                    chosen = true
                    bestRow = s.row
                    bestCol = s.col
                    bestAt = cursor
                }
                cursor = (try Checked.add(cursor, 1))
            }
            if chosen {
                var shownValue: Int64 = 0
                var shownFault: UInt8 = 1
                var u: Int64 = 0
                while u < Int64(truncatingIfNeeded: trail.count) {
                    if (try Checked.at(trail, u)) == bestAt {
                        shownValue = (try Checked.at(trail, (try Checked.add(u, 1))))
                        shownFault = (try Checked.narrow((try Checked.at(trail, (try Checked.add(u, 2)))), to: UInt8.self))
                    }
                    u = (try Checked.add(u, 3))
                }
                let line: SheetShownPayload = SheetShownPayload(row: bestRow, col: bestCol, value: shownValue, fault: shownFault)
                out.append(line)
                started = true
                lastRow = bestRow
                lastCol = bestCol
            } else {
                more = false
            }
        }
        return out
    }
}
