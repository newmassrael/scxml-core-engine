// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
// SCE-MAP: orset_union.scxml:23 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `OrsetUnion`, a file of the
// module that calls it: a call is qualified by the enum
// (`OrsetUnion.orsetUnion(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum OrsetUnion {
    public static func orsetUnion(_ a: [OrsetEntryPayload], _ b: [OrsetEntryPayload]) throws -> [OrsetEntryPayload] {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        var out: [OrsetEntryPayload] = []
        out.reserveCapacity(1024)
        try sceFailure.throwIfFailed()
        for x in a {
            out.append(x)
            try sceFailure.throwIfFailed()
        }
        for y in b {
            var held: Bool = false
            try sceFailure.throwIfFailed()
            for z in a {
                do {
                    let sceCond4 = z.element == y.element && z.wallTime == y.wallTime && z.counter == y.counter && z.nodeId == y.nodeId
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        held = true
                        try sceFailure.throwIfFailed()
                    }
                }
            }
            do {
                let sceCond3 = !held
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    out.append(y)
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
