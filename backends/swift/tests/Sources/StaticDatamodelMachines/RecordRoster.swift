// SCE-GENERATED — DO NOT EDIT
// source-hash: 79bd488e080bbb06d64d257e2fbd5255070d9d17deefefd2dca818e62be7e33d
// SCE-MAP: record_roster.scxml:14 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `RecordRoster`, a file of the
// module that calls it: a call is qualified by the enum
// (`RecordRoster.recordRoster(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum RecordRoster {
    public static func recordRoster(_ roster: [RecordMemberPayload], _ node: UInt64, _ role: UInt8) throws(AlgorithmError) -> [RecordMemberPayload] {
        var out: [RecordMemberPayload] = []
        out.reserveCapacity(64)
        for m in roster {
            if m.node != node {
                out.append(m)
            }
        }
        if role != 0 {
            let made: RecordMemberPayload = RecordMemberPayload(node: node, role: role)
            out.append(made)
        }
        return out
    }
}
