// SCE-GENERATED — DO NOT EDIT
// source-hash: 90dfacafb9c3b3dd4ea571704b12a814c54947805373c06b331ad43ea5f220d0
// SCE-MAP: transform_floor.scxml:42 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A formula per output: a static function of the caseless enum `TransformFloor` for each, a
// file of the module that calls them. A call is qualified by the enum
// (`TransformFloor.<function>(…)`), which a member of the calling type cannot hide as it would
// hide a free function of the same name. An output that reads another output calls its function.

import SceForgeRuntime

public enum TransformFloor {

    public static func computeFloored(_ value: Double) -> Int32 {
        Real.floor(value, to: Int32.self)
    }
}
