// SCE-GENERATED — DO NOT EDIT
// source-hash: b2df42e0f17f5ee4abac1f15e81d26147398511889a9002b5b80cbdc20deacdf
// source: sce:std/merge/orset_observed.scxml
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
    public static func orsetObserved(_ live: [OrsetEntryPayload], _ element: UInt64) throws(AlgorithmError) -> [OrsetEntryPayload] {
        var out: [OrsetEntryPayload] = []
        out.reserveCapacity(1024)
        for x in live {
            if x.element == element {
                out.append(x)
            }
        }
        return out
    }
}
