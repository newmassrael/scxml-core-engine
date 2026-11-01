// SCE-GENERATED — DO NOT EDIT
// source-hash: 1ab1a10b34f4b148294aee29e5cc1a5856f60a5244f882047fc2e31353e34959
// SCE-MAP: transform_extreme_literals.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A formula per output: a static function of the caseless enum `TransformExtremeLiterals` for each, a
// file of the module that calls them. A call is qualified by the enum
// (`TransformExtremeLiterals.<function>(…)`), which a member of the calling type cannot hide as it would
// hide a free function of the same name. An output that reads another output calls its function.

public enum TransformExtremeLiterals {

    public static func computeFlipped(_ word: UInt64) -> UInt64 {
        word ^ 18446744073709551615
    }

    public static func computeLowest(_ word: UInt64) -> Int64 {
        -9223372036854775808
    }
}
