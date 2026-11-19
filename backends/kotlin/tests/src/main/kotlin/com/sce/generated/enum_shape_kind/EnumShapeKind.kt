// SCE-GENERATED — DO NOT EDIT
// source-hash: bfc186601db587ca37c93df75a2683e7ba8b863c255844424d95ac86e99a7d9d
// source: enum_shape_kind.scxml
// SCE-MAP: enum_shape_kind.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.enum_shape_kind

@OptIn(ExperimentalUnsignedTypes::class)
enum class EnumShapeKind(val raw: UByte) {
    DOT(0u.toUByte()),
    CIRCLE(1u.toUByte()),
    DISC(2u.toUByte()),
    RECT(3u.toUByte()),
    LABEL(4u.toUByte()),
    PATH(5u.toUByte()),
    ;

    /** The carrier value this variant declares. */
    fun toUnderlying(): UByte = raw

    /** The name the document declares for this variant. */
    fun declaredName(): String = when (this) {
        DOT -> "dot"
        CIRCLE -> "circle"
        DISC -> "disc"
        RECT -> "rect"
        LABEL -> "label"
        PATH -> "path"
    }

    companion object {
        /**
         * The variant [raw] declares, or null — the declared set is closed,
         * so a value outside it is not a value of this type.
         */
        fun fromUnderlying(raw: UByte): EnumShapeKind? =
            entries.firstOrNull { it.raw == raw }
    }
}
