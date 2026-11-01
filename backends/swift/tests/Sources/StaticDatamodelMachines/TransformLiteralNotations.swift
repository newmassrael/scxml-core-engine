// SCE-GENERATED — DO NOT EDIT
// source-hash: d81d136531455f0f9576fa01a2c502ca5ca47a88aee0b37d226a7f921bef6559
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
