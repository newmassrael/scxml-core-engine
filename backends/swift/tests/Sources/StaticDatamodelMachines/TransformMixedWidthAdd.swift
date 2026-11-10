// SCE-GENERATED — DO NOT EDIT
// source-hash: 982595e044dab87d2a02655ddc0eb9f9f68babd78e0aa16f5305c5e6c8b4eb30
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
