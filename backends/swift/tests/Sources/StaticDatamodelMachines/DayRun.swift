// SCE-GENERATED — DO NOT EDIT
// source-hash: adbae20e9ca5d62b09870f4a9e5e88119b90dbcf69963958bb021c42c5c2c6a4
// SCE-MAP: algorithm_day_run.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `DayRun`, a file of the
// module that calls it: a call is qualified by the enum
// (`DayRun.dayRun(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum DayRun {
    public static func dayRun(_ first: UInt8, _ count: UInt8) throws -> [UInt8] {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        var out: [UInt8] = []
        out.reserveCapacity(8)
        try sceFailure.throwIfFailed()
        var i: UInt8 = 0
        try sceFailure.throwIfFailed()
        while true {
            let sceCond2 = i < count
            try sceFailure.throwIfFailed()
            if !sceCond2 { break }
            out.append(Checked.add(sceFailure, first, i))
            try sceFailure.throwIfFailed()
            i = Checked.add(sceFailure, i, 1)
            try sceFailure.throwIfFailed()
        }
        do {
            let sceReturned: [UInt8] = out
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
