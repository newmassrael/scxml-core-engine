// SCE-GENERATED — DO NOT EDIT
// source-hash: 1ab1a10b34f4b148294aee29e5cc1a5856f60a5244f882047fc2e31353e34959
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
