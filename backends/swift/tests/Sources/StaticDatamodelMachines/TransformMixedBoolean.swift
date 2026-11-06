// SCE-GENERATED — DO NOT EDIT
// source-hash: 7b98612ed0f35af10eaffafb7dd60088b78d6093971dc622b4d3161a336da46c
// SCE-MAP: transform_mixed_boolean.scxml:30 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A formula per output: a static function of the caseless enum `TransformMixedBoolean` for each, a
// file of the module that calls them. A call is qualified by the enum
// (`TransformMixedBoolean.<function>(…)`), which a member of the calling type cannot hide as it would
// hide a free function of the same name. An output that reads another output calls its function.

public enum TransformMixedBoolean {

    public static func computeAlerting(_ alarm: Int32, _ level: Int32) -> Bool {
        alarm == 2 || level >= 1 && level <= 4
    }

    public static func computeRanged(_ alarm: Int32, _ level: Int32) -> Bool {
        level >= 1 && level <= 4 || alarm == 2
    }
}
