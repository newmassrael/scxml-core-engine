// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
// SCE-MAP: mv_value_drop.scxml:16 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `MvValueDrop`, a file of the
// module that calls it: a call is qualified by the enum
// (`MvValueDrop.mvValueDrop(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum MvValueDrop {
    public static func mvValueDrop(_ values: [MvValuePayload], _ gone: MvValuePayload) throws -> [MvValuePayload] {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        var out: [MvValuePayload] = []
        out.reserveCapacity(512)
        try sceFailure.throwIfFailed()
        for v in values {
            do {
                let sceCond3 = v.id != gone.id || v.field != gone.field || v.wallTime != gone.wallTime || v.counter != gone.counter || v.nodeId != gone.nodeId
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    out.append(v)
                    try sceFailure.throwIfFailed()
                }
            }
        }
        do {
            let sceReturned: [MvValuePayload] = out
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
