// SCE-GENERATED — DO NOT EDIT
// source-hash: 9e46d5ba618e2f5a559e7793b0e0ddd1d7ba8428fd3d96128f06266cf4750b56
// SCE-MAP: changes_since.scxml:37 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `ChangesSince`, a file of the
// module that calls it: a call is qualified by the enum
// (`ChangesSince.changesSince(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum ChangesSince {
    public static func changesSince(_ log: [ChangeEntryPayload], _ since: UInt64, _ current: UInt64, _ lowWater: UInt64, _ limit: UInt32) throws(AlgorithmError) -> [ChangeEntryPayload] {
        if !(since >= lowWater && since <= current) {
            throw AlgorithmError.precondition
        }
        if !(limit >= 1 && limit <= 256) {
            throw AlgorithmError.precondition
        }
        var out: [ChangeEntryPayload] = []
        out.reserveCapacity(256)
        var taken: UInt32 = 0
        var prevToken: UInt64 = 0
        var i: UInt32 = 0
        for e in log {
            if !(e.token >= prevToken && e.op >= 1 && e.op <= 3) {
                throw AlgorithmError.precondition
            }
            prevToken = e.token
            if e.token >= since && taken < limit {
                var superseded: Bool = false
                var j: UInt32 = 0
                for f in log {
                    if j > i && f.item == e.item {
                        superseded = true
                    }
                    j = (try Checked.add(j, 1))
                }
                if !superseded {
                    out.append(e)
                    taken = (try Checked.add(taken, 1))
                }
            }
            i = (try Checked.add(i, 1))
        }
        return out
    }
}
