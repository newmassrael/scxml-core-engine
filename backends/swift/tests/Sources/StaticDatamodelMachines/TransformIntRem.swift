// SCE-GENERATED — DO NOT EDIT
// source-hash: adac8824ce3bf12969cf8a9f4a1db094aa59b5b7a2d7b37b7a611f3c7eb217dc
// SCE-MAP: transform_int_rem.scxml:16 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A formula per output: a static function of the caseless enum `TransformIntRem` for each, a
// file of the module that calls them. A call is qualified by the enum
// (`TransformIntRem.<function>(…)`), which a member of the calling type cannot hide as it would
// hide a free function of the same name. An output that reads another output calls its function.

public enum TransformIntRem {

    public static func computeRemainder(_ dividend: Int32, _ divisor: Int32) -> Int32 {
        dividend % divisor
    }
}
