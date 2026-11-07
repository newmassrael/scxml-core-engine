// SCE-GENERATED — DO NOT EDIT
// source-hash: b2921c03007062914a1e69cc4069c6b54c8d66f97313a50f3246eed3d071c790
// SCE-MAP: event_later.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `EventLater`, a file of the
// module that calls it: a call is qualified by the enum
// (`EventLater.eventLater(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum EventLater {
    public static func eventLater(_ wall: Int64, _ counter: UInt32, _ node: UInt64, _ bestWall: Int64, _ bestCounter: UInt32, _ bestNode: UInt64) -> Bool {
        return wall > bestWall || wall == bestWall && (counter > bestCounter || counter == bestCounter && node > bestNode)
    }
}
