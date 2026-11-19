// SCE-GENERATED — DO NOT EDIT
// source-hash: c8dbd7025e6276c104b2cdfc73ad2f4136c50815d80807258fdeded7459f2b79
// source: shape_kind.scxml
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
