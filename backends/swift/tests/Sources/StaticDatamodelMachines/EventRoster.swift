// SCE-GENERATED — DO NOT EDIT
// source-hash: 1178ba59796ca1f7c46f8569fd4bfda85c2b97db65f51728f3615d990d97dd1c
// SCE-MAP: event_roster.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `EventRoster`, a file of the
// module that calls it: a call is qualified by the enum
// (`EventRoster.eventRoster(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum EventRoster {
    public static func eventRoster(_ roster: [EventMemberPayload], _ node: UInt64, _ role: UInt8) throws -> [EventMemberPayload] {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        var out: [EventMemberPayload] = []
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
                let made: EventMemberPayload = EventMemberPayload(node: node, role: role)
                try sceFailure.throwIfFailed()
                out.append(made)
                try sceFailure.throwIfFailed()
            }
        }
        do {
            let sceReturned: [EventMemberPayload] = out
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
