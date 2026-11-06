// SCE-GENERATED — DO NOT EDIT
// source-hash: 7b98612ed0f35af10eaffafb7dd60088b78d6093971dc622b4d3161a336da46c
// SCE-MAP: transform_guarded_div.scxml:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A formula per output: a static function of the caseless enum `TransformGuardedDiv` for each, a
// file of the module that calls them. A call is qualified by the enum
// (`TransformGuardedDiv.<function>(…)`), which a member of the calling type cannot hide as it would
// hide a free function of the same name. An output that reads another output calls its function.

public enum TransformGuardedDiv {

    public static func computeShare(_ dividend: Int32, _ divisor: Int32) -> Int32 {
        divisor != 0 ? dividend / divisor : 0
    }
}
