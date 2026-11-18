// SCE-GENERATED — DO NOT EDIT
// source-hash: a9c299ab96f6e6c2a694137962a3f363e82399bf22fbd9f4aa0018913d55ad58
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
