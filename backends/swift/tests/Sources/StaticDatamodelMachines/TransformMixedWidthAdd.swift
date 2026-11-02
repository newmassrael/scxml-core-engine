// SCE-GENERATED — DO NOT EDIT
// source-hash: 814c7782483a2e1383636d59b115ce52cb83fc95e5c66e5ef4b90acbc7ce1d0e
// SCE-MAP: transform_mixed_width_add.scxml:17 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A formula per output: a static function of the caseless enum `TransformMixedWidthAdd` for each, a
// file of the module that calls them. A call is qualified by the enum
// (`TransformMixedWidthAdd.<function>(…)`), which a member of the calling type cannot hide as it would
// hide a free function of the same name. An output that reads another output calls its function.

public enum TransformMixedWidthAdd {

    public static func computeTotal(_ base: Int64, _ offset: UInt8) -> Int64 {
        base + Int64(offset)
    }
}
