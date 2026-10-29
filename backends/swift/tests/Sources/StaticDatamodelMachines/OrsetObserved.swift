// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
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
