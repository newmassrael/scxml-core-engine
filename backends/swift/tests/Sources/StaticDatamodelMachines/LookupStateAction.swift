// SCE-GENERATED — DO NOT EDIT
// source-hash: eeb0103caca15ce30c6db8ca9e98ae382eb1228e8e1148f2aec785510af4fbde
// SCE-MAP: lookup_state_action.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// A table read by exact match: a static function of the caseless enum `LookupStateAction`, a
// file of the module that calls it. A call is qualified by the enum
// (`LookupStateAction.lookupAction(…)`), which a member of the calling type cannot hide as
// it would hide a free function of the same name. A key the table
// does not list answers no value: the call is an error, and the function answers `nil`.
import SceForgeRuntime

public enum LookupStateAction {
    private static let sceKeys: [Int32] = [0, 1, 2, 3]
    private static let sceValues: [Int32] = [10, 20, 30, 40]

    public static func lookupAction(_ state: Int32) -> Int32? {
        SceForgeRuntime.Lookup.find(sceKeys, sceValues, state)
    }
}
