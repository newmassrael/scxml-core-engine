// SCE-GENERATED — DO NOT EDIT
// source-hash: d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b
// SCE-MAP: paint_color.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.paint_color

@OptIn(ExperimentalUnsignedTypes::class)
enum class PaintColor(val raw: UByte) {
    RED(0u.toUByte()),
    GREEN(1u.toUByte()),
    BLUE(2u.toUByte()),
    ;

    /** The carrier value this variant declares. */
    fun toUnderlying(): UByte = raw

    /** The name the document declares for this variant. */
    fun declaredName(): String = when (this) {
        RED -> "red"
        GREEN -> "green"
        BLUE -> "blue"
    }

    companion object {
        /**
         * The variant [raw] declares, or null — the declared set is closed,
         * so a value outside it is not a value of this type.
         */
        fun fromUnderlying(raw: UByte): PaintColor? =
            entries.firstOrNull { it.raw == raw }
    }
}
