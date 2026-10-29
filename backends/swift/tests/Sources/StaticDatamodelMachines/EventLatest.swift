// SCE-GENERATED — DO NOT EDIT
// source-hash: 047dc621dfd01b6348575becd26162c58fa20fa7680f076526766f91e97d2c5f
// SCE-MAP: event_latest.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `EventLatest`, a file of the
// module that calls it: a call is qualified by the enum
// (`EventLatest.eventLatest(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum EventLatest {
    public static func eventLatest(_ values: [MvValuePayload], _ id: UInt64, _ field: UInt64, _ none: Int64) -> Int64 {
        var found: Bool = false
        var bestWall: Int64 = 0
        var bestCounter: UInt32 = 0
        var bestNode: UInt64 = 0
        var best: Int64 = none
        for v in values {
            if v.id == id && v.field == field && (!found || EventLater.eventLater(v.wallTime, v.counter, v.nodeId, bestWall, bestCounter, bestNode)) {
                found = true
                bestWall = v.wallTime
                bestCounter = v.counter
                bestNode = v.nodeId
                best = v.value
            }
        }
        return best
    }
}
