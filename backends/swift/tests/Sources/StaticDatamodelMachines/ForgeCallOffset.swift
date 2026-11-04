// SCE-GENERATED — DO NOT EDIT
// source-hash: ea85fee4970d20335e2c3e2f4a0cd89a9f4f0a21d54f6dae4cb7a7218e2be4b5
// SCE-MAP: forge_call_offset.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A formula per output: a static function of the caseless enum `ForgeCallOffset` for each, a
// file of the module that calls them. A call is qualified by the enum
// (`ForgeCallOffset.<function>(…)`), which a member of the calling type cannot hide as it would
// hide a free function of the same name. An output that reads another output calls its function.

public enum ForgeCallOffset {

    public static func computeOffset(_ level: Int32) -> Int32 {
        level - 10
    }
}
