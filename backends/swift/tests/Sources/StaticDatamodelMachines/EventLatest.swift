// SCE-GENERATED — DO NOT EDIT
// source-hash: 81a3de342b33c3189f4a5813894163bc1e16bfc77aa7ead74e709e8ba7b570b9
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
