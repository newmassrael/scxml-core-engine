// SCE-GENERATED — DO NOT EDIT
// source-hash: 62067cbe5198ad923571ee5243de08cf004a9a483bf54ae3e6db2a14e8ad77a7
// SCE-MAP: transform_int_ratio.scxml:20 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A formula per output: a static function of the caseless enum `TransformIntRatio` for each, a
// file of the module that calls them. A call is qualified by the enum
// (`TransformIntRatio.<function>(…)`), which a member of the calling type cannot hide as it would
// hide a free function of the same name. An output that reads another output calls its function.

public enum TransformIntRatio {

    public static func computeRatio(_ numerator: Int32, _ denominator: Int32) -> Double {
        Double(numerator) / Double(denominator)
    }
}
