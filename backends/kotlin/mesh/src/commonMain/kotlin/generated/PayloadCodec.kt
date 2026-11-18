// SCE-GENERATED — DO NOT EDIT
// source-hash: 687f4c0ae3532ecabd8fec6c87388a5c95d9c36571dc803cf9f1b82e7d19a7ab
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
