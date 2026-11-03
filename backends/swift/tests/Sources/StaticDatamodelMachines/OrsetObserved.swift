// SCE-GENERATED — DO NOT EDIT
// source-hash: fb4f42a34e7ac40a28d8e5daee5d633c3c45dddac513816edf2bb83fdc85f91c
// SCE-MAP: orset_observed.scxml:15 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `OrsetObserved`, a file of the
// module that calls it: a call is qualified by the enum
// (`OrsetObserved.orsetObserved(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum OrsetObserved {
    public static func orsetObserved(_ live: [OrsetEntryPayload], _ element: UInt64) throws -> [OrsetEntryPayload] {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        var out: [OrsetEntryPayload] = []
        out.reserveCapacity(1024)
        try sceFailure.throwIfFailed()
        for x in live {
            do {
                let sceCond3 = x.element == element
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
