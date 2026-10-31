// SCE-GENERATED — DO NOT EDIT
// source-hash: 90dfacafb9c3b3dd4ea571704b12a814c54947805373c06b331ad43ea5f220d0
// SCE-MAP: algorithm_weekly_expand.scxml:19 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `AlgorithmWeeklyExpand`, a file of the
// module that calls it: a call is qualified by the enum
// (`AlgorithmWeeklyExpand.algorithmWeeklyExpand(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum AlgorithmWeeklyExpand {
    public static func algorithmWeeklyExpand(_ dtstart: Int64, _ mask: UInt8, _ interval: Int64, _ window_start: Int64, _ window_end: Int64) throws -> [Int64] {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        var out: [Int64] = []
        out.reserveCapacity(366)
        try sceFailure.throwIfFailed()
        var d: Int64 = window_start > dtstart ? window_start : dtstart
        try sceFailure.throwIfFailed()
        while true {
            let sceCond2 = d < window_end
            try sceFailure.throwIfFailed()
            if !sceCond2 { break }
            let weekday: Int64 = Checked.rem(sceFailure, Checked.add(sceFailure, d, 3), 7)
            try sceFailure.throwIfFailed()
            let week: Int64 = Checked.div(sceFailure, Checked.sub(sceFailure, d, dtstart), 7)
            try sceFailure.throwIfFailed()
            do {
                let sceCond3 = ((Int64(mask) >> weekday) & 1) == 1 && Checked.rem(sceFailure, week, interval) == 0
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    out.append(d)
                    try sceFailure.throwIfFailed()
                }
            }
            d = Checked.add(sceFailure, d, 1)
            try sceFailure.throwIfFailed()
        }
        do {
            let sceReturned: [Int64] = out
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
