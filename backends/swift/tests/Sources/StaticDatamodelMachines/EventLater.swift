// SCE-GENERATED — DO NOT EDIT
// source-hash: a959df7d40a6301aa78e8501513db4064b02c0155bca6d82b638d4f2f72d3815
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
