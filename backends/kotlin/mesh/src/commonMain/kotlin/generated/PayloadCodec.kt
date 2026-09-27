// SCE-GENERATED — DO NOT EDIT
// source-hash: 418438a744050ac9cd6cb0691018fdb0ef433f86cf92fa1a16f148307512f21e
// SCE-MAP: payload_codec.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.payload_codec

@OptIn(ExperimentalUnsignedTypes::class)
enum class PayloadCodec(val raw: UByte) {
    NONE(0u.toUByte()),
    JSON(1u.toUByte()),
    CBOR(2u.toUByte()),
    TYPED(3u.toUByte()),
    RAW(4u.toUByte()),
    ;

    /** The carrier value this variant declares. */
    fun toUnderlying(): UByte = raw

    companion object {
        /**
         * The variant [raw] declares, or null — the declared set is closed,
         * so a value outside it is not a value of this type.
         */
        fun fromUnderlying(raw: UByte): PayloadCodec? =
            entries.firstOrNull { it.raw == raw }
    }
}
