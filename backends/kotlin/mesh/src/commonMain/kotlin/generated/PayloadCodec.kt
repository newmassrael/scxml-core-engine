// SCE-GENERATED — DO NOT EDIT
// source-hash: c99c3c0939ed7b1256239958164f8fd91e9fc2a690644b8df67efbcf58589a94
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

    /** The name the document declares for this variant. */
    fun declaredName(): String = when (this) {
        NONE -> "none"
        JSON -> "json"
        CBOR -> "cbor"
        TYPED -> "typed"
        RAW -> "raw"
    }

    companion object {
        /**
         * The variant [raw] declares, or null — the declared set is closed,
         * so a value outside it is not a value of this type.
         */
        fun fromUnderlying(raw: UByte): PayloadCodec? =
            entries.firstOrNull { it.raw == raw }
    }
}
