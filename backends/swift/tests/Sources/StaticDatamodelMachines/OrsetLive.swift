// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
// SCE-MAP: orset_live.scxml:31 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `OrsetLive`, a file of the
// module that calls it: a call is qualified by the enum
// (`OrsetLive.orsetLive(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum OrsetLive {
    public static func orsetLive(_ adds: [OrsetEntryPayload], _ tombstones: [OrsetEntryPayload]) throws -> [OrsetEntryPayload] {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        var out: [OrsetEntryPayload] = []
        out.reserveCapacity(1024)
        try sceFailure.throwIfFailed()
        for x in adds {
            var removed: Bool = false
            try sceFailure.throwIfFailed()
            for t in tombstones {
                do {
                    let sceCond4 = t.element == x.element && t.wallTime == x.wallTime && t.counter == x.counter && t.nodeId == x.nodeId
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        removed = true
                        try sceFailure.throwIfFailed()
                    }
                }
            }
            do {
                let sceCond3 = !removed
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    out.append(x)
                    try sceFailure.throwIfFailed()
                }
            }
        }
        do {
            let sceReturned: [OrsetEntryPayload] = out
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
