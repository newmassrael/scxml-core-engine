// SCE-GENERATED — DO NOT EDIT
// source-hash: 62067cbe5198ad923571ee5243de08cf004a9a483bf54ae3e6db2a14e8ad77a7
// SCE-MAP: algorithm_checked_index.scxml:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `AlgorithmCheckedIndex`, a file of the
// module that calls it: a call is qualified by the enum
// (`AlgorithmCheckedIndex.algorithmCheckedIndex(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum AlgorithmCheckedIndex {
    public static func algorithmCheckedIndex(_ data: [UInt8], _ i: Int32, _ u: UInt32) throws -> UInt8 {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        do {
            let sceReturned: UInt8 = Checked.at(sceFailure, data, i, fallback: 0) ^ Checked.at(sceFailure, data, u, fallback: 0)
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
