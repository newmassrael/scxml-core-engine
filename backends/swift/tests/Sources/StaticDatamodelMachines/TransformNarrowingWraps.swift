// SCE-GENERATED — DO NOT EDIT
// source-hash: 62067cbe5198ad923571ee5243de08cf004a9a483bf54ae3e6db2a14e8ad77a7
// SCE-MAP: transform_narrowing_wraps.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A formula per output: a static function of the caseless enum `TransformNarrowingWraps` for each, a
// file of the module that calls them. A call is qualified by the enum
// (`TransformNarrowingWraps.<function>(…)`), which a member of the calling type cannot hide as it would
// hide a free function of the same name. An output that reads another output calls its function.

public enum TransformNarrowingWraps {

    public static func computeLow(_ wide: UInt32, _ signedWide: Int32, _ big: Int64) -> UInt8 {
        UInt8(truncatingIfNeeded: wide)
    }

    public static func computeLowFromSigned(_ wide: UInt32, _ signedWide: Int32, _ big: Int64) -> UInt8 {
        UInt8(truncatingIfNeeded: signedWide)
    }

    public static func computeNarrowSigned(_ wide: UInt32, _ signedWide: Int32, _ big: Int64) -> Int32 {
        Int32(truncatingIfNeeded: big)
    }
}
