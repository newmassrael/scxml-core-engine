// SCE-GENERATED — DO NOT EDIT
// source-hash: 1178ba59796ca1f7c46f8569fd4bfda85c2b97db65f51728f3615d990d97dd1c
// SCE-MAP: event_conflict.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `EventConflict`, a file of the
// module that calls it: a call is qualified by the enum
// (`EventConflict.eventConflict(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum EventConflict {
    public static func eventConflict(_ values: [MvValuePayload], _ id: UInt64) -> Bool {
        var twice: Bool = false
        for a in values {
            if a.id == id && (a.field == 9 || a.field >= 15 && a.field <= 58) {
                for b in values {
                    if b.id == id && b.field == a.field && (b.wallTime != a.wallTime || b.counter != a.counter || b.nodeId != a.nodeId) {
                        twice = true
                    }
                }
            }
        }
        return twice
    }
}
