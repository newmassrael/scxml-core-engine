// SCE-GENERATED — DO NOT EDIT
// source-hash: a657d6d390827c00a8c65cc79818ec59a776eff477a713478c6b36adae5a7d61
// SCE-MAP: transform_literal_notations.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A formula per output: a static function of the caseless enum `TransformLiteralNotations` for each, a
// file of the module that calls them. A call is qualified by the enum
// (`TransformLiteralNotations.<function>(…)`), which a member of the calling type cannot hide as it would
// hide a free function of the same name. An output that reads another output calls its function.

public enum TransformLiteralNotations {

    public static func computeLowOctal(_ flags: UInt8) -> UInt8 {
        flags & 0xF
    }

    public static func computeEvenBinary(_ flags: UInt8) -> UInt8 {
        flags & 0xA
    }

    public static func computeHighHex(_ flags: UInt8) -> UInt8 {
        flags & 0xF0
    }
}
