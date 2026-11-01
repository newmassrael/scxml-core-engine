// SCE-GENERATED — DO NOT EDIT
// source-hash: eeb0103caca15ce30c6db8ca9e98ae382eb1228e8e1148f2aec785510af4fbde
// SCE-MAP: algorithm_checked_narrow.scxml:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `AlgorithmCheckedNarrow`, a file of the
// module that calls it: a call is qualified by the enum
// (`AlgorithmCheckedNarrow.algorithmCheckedNarrow(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum AlgorithmCheckedNarrow {
    public static func algorithmCheckedNarrow(_ x: Int64, _ y: UInt32) throws -> Int32 {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        let small: UInt8 = Checked.narrow(sceFailure, y, to: UInt8.self)
        try sceFailure.throwIfFailed()
        let wide: Int64 = Checked.add(sceFailure, x, Int64(small))
        try sceFailure.throwIfFailed()
        do {
            let sceReturned: Int32 = Checked.narrow(sceFailure, wide, to: Int32.self)
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
