// SCE-GENERATED — DO NOT EDIT
// source-hash: 1ab1a10b34f4b148294aee29e5cc1a5856f60a5244f882047fc2e31353e34959
// SCE-MAP: transform_bitwise.scxml:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A formula per output: a static function of the caseless enum `TransformBitwise` for each, a
// file of the module that calls them. A call is qualified by the enum
// (`TransformBitwise.<function>(…)`), which a member of the calling type cannot hide as it would
// hide a free function of the same name. An output that reads another output calls its function.

public enum TransformBitwise {

    public static func computeHighNibble(_ byte: UInt8) -> UInt8 {
        (byte >> 4) & 0x0F
    }

    public static func computeLowNibble(_ byte: UInt8) -> UInt8 {
        byte & 0x0F
    }
}
