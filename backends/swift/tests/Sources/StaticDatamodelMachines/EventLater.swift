// SCE-GENERATED — DO NOT EDIT
// source-hash: ea764ab5524bc7a7e7d4553640b3fb26547d13f0d2053c0f5e7214161efbc6d0
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
