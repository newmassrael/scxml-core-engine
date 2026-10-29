// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
// SCE-MAP: mv_value_keep.scxml:22 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `MvValueKeep`, a file of the
// module that calls it: a call is qualified by the enum
// (`MvValueKeep.mvValueKeep(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum MvValueKeep {
    public static func mvValueKeep(_ values: [MvValuePayload], _ held: MvValuePayload) throws -> [MvValuePayload] {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        var out: [MvValuePayload] = []
        out.reserveCapacity(512)
        try sceFailure.throwIfFailed()
        var there: Bool = false
        try sceFailure.throwIfFailed()
        for v in values {
            do {
                let sceCond3 = v.id == held.id && v.field == held.field && v.wallTime == held.wallTime && v.counter == held.counter && v.nodeId == held.nodeId
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    there = true
                    try sceFailure.throwIfFailed()
                }
            }
            out.append(v)
            try sceFailure.throwIfFailed()
        }
        do {
            let sceCond2 = !there
            try sceFailure.throwIfFailed()
            if sceCond2 {
                out.append(held)
                try sceFailure.throwIfFailed()
            }
        }
        do {
            let sceReturned: [MvValuePayload] = out
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
