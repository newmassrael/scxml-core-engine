// SCE-GENERATED — DO NOT EDIT
// source-hash: a6c4b7942ba60b5b0c14ac6360bab7a849287a18564bb536c0e5adccfa0cf705
// source: transform_mixed_width_or.scxml
// SCE-MAP: transform_mixed_width_or.scxml:13 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A formula per output: a static function of the caseless enum `TransformMixedWidthOr` for each, a
// file of the module that calls them. A call is qualified by the enum
// (`TransformMixedWidthOr.<function>(…)`), which a member of the calling type cannot hide as it would
// hide a free function of the same name. An output that reads another output calls its function.

public enum TransformMixedWidthOr {

    public static func computeMerged(_ word: Int64, _ flags: UInt8) -> Int64 {
        word | Int64(flags)
    }
}
