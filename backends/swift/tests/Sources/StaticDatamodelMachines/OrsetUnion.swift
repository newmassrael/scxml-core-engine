// SCE-GENERATED — DO NOT EDIT
// source-hash: fb4f42a34e7ac40a28d8e5daee5d633c3c45dddac513816edf2bb83fdc85f91c
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
