// SCE-GENERATED — DO NOT EDIT
// source-hash: d8c75e4d0233623996dc9a7cc3bdbe850af8d4f91ab023212ebd9f938cf7955f
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
    public static func eventRoster(_ roster: [EventMemberPayload], _ node: UInt64, _ role: UInt8) throws(AlgorithmError) -> [EventMemberPayload] {
        var out: [EventMemberPayload] = []
        out.reserveCapacity(64)
        for m in roster {
            if m.node != node {
                out.append(m)
            }
        }
        if role != 0 {
            let made: EventMemberPayload = EventMemberPayload(node: node, role: role)
            out.append(made)
        }
        return out
    }
}
