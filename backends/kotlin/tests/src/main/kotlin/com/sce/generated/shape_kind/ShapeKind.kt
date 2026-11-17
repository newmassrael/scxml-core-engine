// SCE-GENERATED — DO NOT EDIT
// source-hash: 5ba52a313a5f53415f5c7e49a957fa6eb851fa976ae81a41db1540add4319d51
// SCE-MAP: shape_kind.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.shape_kind

@OptIn(ExperimentalUnsignedTypes::class)
enum class ShapeKind(val raw: UByte) {
    DOT(0u.toUByte()),
    CIRCLE(1u.toUByte()),
    RECT(2u.toUByte()),
    ;

    /** The carrier value this variant declares. */
    fun toUnderlying(): UByte = raw

    /** The name the document declares for this variant. */
    fun declaredName(): String = when (this) {
        DOT -> "dot"
        CIRCLE -> "circle"
        RECT -> "rect"
    }

    companion object {
        /**
         * The variant [raw] declares, or null — the declared set is closed,
         * so a value outside it is not a value of this type.
         */
        fun fromUnderlying(raw: UByte): ShapeKind? =
            entries.firstOrNull { it.raw == raw }
    }
}
