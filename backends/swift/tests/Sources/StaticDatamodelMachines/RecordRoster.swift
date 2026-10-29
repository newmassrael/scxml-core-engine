// SCE-GENERATED — DO NOT EDIT
// source-hash: 634c022b7eb332297dcb6115177eef8cafdb472ead988f0bdc39838cb7f8d8ae
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
    public static func recordRoster(_ roster: [RecordMemberPayload], _ node: UInt64, _ role: UInt8) throws -> [RecordMemberPayload] {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        var out: [RecordMemberPayload] = []
        out.reserveCapacity(64)
        try sceFailure.throwIfFailed()
        for m in roster {
            do {
                let sceCond3 = m.node != node
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    out.append(m)
                    try sceFailure.throwIfFailed()
                }
            }
        }
        do {
            let sceCond2 = role != 0
            try sceFailure.throwIfFailed()
            if sceCond2 {
                let made: RecordMemberPayload = RecordMemberPayload(node: node, role: role)
                try sceFailure.throwIfFailed()
                out.append(made)
                try sceFailure.throwIfFailed()
            }
        }
        do {
            let sceReturned: [RecordMemberPayload] = out
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
