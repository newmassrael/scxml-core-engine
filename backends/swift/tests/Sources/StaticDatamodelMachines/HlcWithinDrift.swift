// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
// SCE-MAP: hlc_within_drift.scxml:23 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `HlcWithinDrift`, a file of the
// module that calls it: a call is qualified by the enum
// (`HlcWithinDrift.hlcWithinDrift(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum HlcWithinDrift {
    public static func hlcWithinDrift(_ wallTime: Int64, _ now: Int64, _ maxDrift: Int64) throws -> Bool {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        do {
            let sceCond2 = !(maxDrift >= 0)
            try sceFailure.throwIfFailed()
            if sceCond2 {
                throw AlgorithmError.precondition
            }
        }
        do {
            let sceCond2 = wallTime <= now
            try sceFailure.throwIfFailed()
            if sceCond2 {
                do {
                    let sceReturned: Bool = true
                    try sceFailure.throwIfFailed()
                    return sceReturned
                }
            }
        }
        do {
            let sceReturned: Bool = Checked.sub(sceFailure, wallTime, now) <= maxDrift
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
