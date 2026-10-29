// SCE-GENERATED — DO NOT EDIT
// source-hash: 68d77b5e0c96e361239e1512887ba2ef6d35bfab0188bf0e2afbd53ca387f3bc
// SCE-MAP: utc_offset_at.scxml:25 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `UtcOffsetAt`, a file of the
// module that calls it: a call is qualified by the enum
// (`UtcOffsetAt.utcOffsetAt(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum UtcOffsetAt {
    public static func utcOffsetAt(_ starts: [Int64], _ offsets: [Int32], _ t: Int64) throws -> Int32 {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        do {
            let sceCond2 = !(starts.count == offsets.count && starts.count > 0)
            try sceFailure.throwIfFailed()
            if sceCond2 {
                throw AlgorithmError.precondition
            }
        }
        do {
            let sceCond2 = !(t >= Checked.at(sceFailure, starts, 0, fallback: 0))
            try sceFailure.throwIfFailed()
            if sceCond2 {
                throw AlgorithmError.precondition
            }
        }
        var k: UInt32 = 0
        try sceFailure.throwIfFailed()
        var i: UInt32 = 1
        try sceFailure.throwIfFailed()
        while true {
            let sceCond2 = i < UInt32(truncatingIfNeeded: starts.count)
            try sceFailure.throwIfFailed()
            if !sceCond2 { break }
            do {
                let sceCond3 = !(Checked.at(sceFailure, starts, i, fallback: 0) > Checked.at(sceFailure, starts, Checked.sub(sceFailure, i, 1), fallback: 0))
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    throw AlgorithmError.precondition
                }
            }
            do {
                let sceCond3 = Checked.at(sceFailure, starts, i, fallback: 0) <= t
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    k = i
                    try sceFailure.throwIfFailed()
                }
            }
            i = Checked.add(sceFailure, i, 1)
            try sceFailure.throwIfFailed()
        }
        do {
            let sceReturned: Int32 = Checked.at(sceFailure, offsets, k, fallback: 0)
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
