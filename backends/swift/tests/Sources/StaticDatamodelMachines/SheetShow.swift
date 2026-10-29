// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
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
    public static func sheetShow(_ cells: [LwwCellPayload], _ trail: [Int64]) throws -> [SheetShownPayload] {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        var out: [SheetShownPayload] = []
        out.reserveCapacity(256)
        try sceFailure.throwIfFailed()
        var started: Bool = false
        try sceFailure.throwIfFailed()
        var lastRow: UInt64 = 0
        try sceFailure.throwIfFailed()
        var lastCol: UInt64 = 0
        try sceFailure.throwIfFailed()
        var more: Bool = true
        try sceFailure.throwIfFailed()
        while true {
            let sceCond2 = more
            try sceFailure.throwIfFailed()
            if !sceCond2 { break }
            var chosen: Bool = false
            try sceFailure.throwIfFailed()
            var bestRow: UInt64 = 0
            try sceFailure.throwIfFailed()
            var bestCol: UInt64 = 0
            try sceFailure.throwIfFailed()
            var bestAt: Int64 = 0
            try sceFailure.throwIfFailed()
            var cursor: Int64 = 0
            try sceFailure.throwIfFailed()
            for s in cells {
                do {
                    let sceCond4 = (!started || s.row > lastRow || s.row == lastRow && s.col > lastCol) && (!chosen || s.row < bestRow || s.row == bestRow && s.col < bestCol)
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        chosen = true
                        try sceFailure.throwIfFailed()
                        bestRow = s.row
                        try sceFailure.throwIfFailed()
                        bestCol = s.col
                        try sceFailure.throwIfFailed()
                        bestAt = cursor
                        try sceFailure.throwIfFailed()
                    }
                }
                cursor = Checked.add(sceFailure, cursor, 1)
                try sceFailure.throwIfFailed()
            }
            do {
                let sceCond3 = chosen
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    var shownValue: Int64 = 0
                    try sceFailure.throwIfFailed()
                    var shownFault: UInt8 = 1
                    try sceFailure.throwIfFailed()
                    var u: Int64 = 0
                    try sceFailure.throwIfFailed()
                    while true {
                        let sceCond5 = u < Int64(truncatingIfNeeded: trail.count)
                        try sceFailure.throwIfFailed()
                        if !sceCond5 { break }
                        do {
                            let sceCond6 = Checked.at(sceFailure, trail, u, fallback: 0) == bestAt
                            try sceFailure.throwIfFailed()
                            if sceCond6 {
                                shownValue = Checked.at(sceFailure, trail, Checked.add(sceFailure, u, 1), fallback: 0)
                                try sceFailure.throwIfFailed()
                                shownFault = Checked.narrow(sceFailure, Checked.at(sceFailure, trail, Checked.add(sceFailure, u, 2), fallback: 0), to: UInt8.self)
                                try sceFailure.throwIfFailed()
                            }
                        }
                        u = Checked.add(sceFailure, u, 3)
                        try sceFailure.throwIfFailed()
                    }
                    let line: SheetShownPayload = SheetShownPayload(row: bestRow, col: bestCol, value: shownValue, fault: shownFault)
                    try sceFailure.throwIfFailed()
                    out.append(line)
                    try sceFailure.throwIfFailed()
                    started = true
                    try sceFailure.throwIfFailed()
                    lastRow = bestRow
                    try sceFailure.throwIfFailed()
                    lastCol = bestCol
                    try sceFailure.throwIfFailed()
                } else {
                    more = false
                    try sceFailure.throwIfFailed()
                }
            }
        }
        do {
            let sceReturned: [SheetShownPayload] = out
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
