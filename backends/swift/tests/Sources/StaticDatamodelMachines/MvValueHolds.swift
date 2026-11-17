// SCE-GENERATED — DO NOT EDIT
// source-hash: 451ed84f90f73b42f48909eca6f7a302d6cb0b852dd68c846f5e713406957876
// SCE-MAP: mv_value_holds.scxml:13 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `MvValueHolds`, a file of the
// module that calls it: a call is qualified by the enum
// (`MvValueHolds.mvValueHolds(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum MvValueHolds {
    public static func mvValueHolds(_ values: [MvValuePayload], _ probe: MvValuePayload) -> Bool {
        var found: Bool = false
        for v in values {
            if v.id == probe.id && v.field == probe.field && v.wallTime == probe.wallTime && v.counter == probe.counter && v.nodeId == probe.nodeId {
                found = true
            }
        }
        return found
    }
}
