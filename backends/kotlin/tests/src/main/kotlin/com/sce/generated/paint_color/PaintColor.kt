// SCE-GENERATED — DO NOT EDIT
// source-hash: 7973737417cb8ea47734770193e07481b6487f8ce008338d6d9a4887b3c44cb5
// source: paint_color.scxml
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
