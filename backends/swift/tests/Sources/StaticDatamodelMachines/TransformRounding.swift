// SCE-GENERATED — DO NOT EDIT
// source-hash: 88c3d475be4b704e681e435f769b6c1e874023c075725ce0d093a310a81ad11d
// SCE-MAP: transform_rounding.scxml:37 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A formula per output: a static function of the caseless enum `TransformRounding` for each, a
// file of the module that calls them. A call is qualified by the enum
// (`TransformRounding.<function>(…)`), which a member of the calling type cannot hide as it would
// hide a free function of the same name. An output that reads another output calls its function.

import SceForgeRuntime

public enum TransformRounding {

    public static func computeRounded(_ value: Double) -> Int32 {
        Real.round(value, to: Int32.self)
    }
}
