// SCE-GENERATED — DO NOT EDIT
// source-hash: b2921c03007062914a1e69cc4069c6b54c8d66f97313a50f3246eed3d071c790
// SCE-MAP: algorithm_day_repeat.scxml:13 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `DayRepeat`, a file of the
// module that calls it: a call is qualified by the enum
// (`DayRepeat.dayRepeat(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum DayRepeat {
    public static func dayRepeat(_ value: UInt8, _ count: UInt8) throws -> [UInt8] {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        var out: [UInt8] = []
        out.reserveCapacity(4)
        try sceFailure.throwIfFailed()
        var i: UInt8 = 0
        try sceFailure.throwIfFailed()
        while true {
            let sceCond2 = i < count
            try sceFailure.throwIfFailed()
            if !sceCond2 { break }
            out.append(value)
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
