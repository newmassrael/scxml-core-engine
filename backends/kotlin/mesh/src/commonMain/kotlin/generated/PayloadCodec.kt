// SCE-GENERATED — DO NOT EDIT
// source-hash: 58f5a17e479c571b0f1a95c9ced6b8f03b5d957325be03d16f15d59d99727b1b
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
