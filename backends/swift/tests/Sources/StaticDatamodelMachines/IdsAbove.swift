// SCE-GENERATED — DO NOT EDIT
// source-hash: 78a4af41fcfe4b9fe02872a2af9a8154276b74fe73d9b0e67db5af9da8c38837
// source: algorithm_ids_above.scxml
// SCE-MAP: algorithm_ids_above.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `IdsAbove`, a file of the
// module that calls it: a call is qualified by the enum
// (`IdsAbove.idsAbove(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum IdsAbove {
    public static func idsAbove(_ ids: [UInt32], _ floor: UInt32) -> Bool {
        var above: Bool = false
        for id in ids {
            if id > floor {
                above = true
            }
        }
        return above
    }
}
