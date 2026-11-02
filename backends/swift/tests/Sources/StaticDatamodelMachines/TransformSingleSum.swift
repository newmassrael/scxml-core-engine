// SCE-GENERATED — DO NOT EDIT
// source-hash: f52959f8eb76029a8e6cff0fae9c1d7f0469eb21cd52301f306b53afbc606d1a
// SCE-MAP: transform_single_sum.scxml:24 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A formula per output: a static function of the caseless enum `TransformSingleSum` for each, a
// file of the module that calls them. A call is qualified by the enum
// (`TransformSingleSum.<function>(…)`), which a member of the calling type cannot hide as it would
// hide a free function of the same name. An output that reads another output calls its function.

public enum TransformSingleSum {

    public static func computeTotal(_ level: Float, _ step: Float) -> Float {
        level + step
    }
}
