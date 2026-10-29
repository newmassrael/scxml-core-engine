// SCE-GENERATED — DO NOT EDIT
// source-hash: 75c0c87af90d82dec15f516a721dd486d47e1280589aefe06f15efdef5f4f850
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
