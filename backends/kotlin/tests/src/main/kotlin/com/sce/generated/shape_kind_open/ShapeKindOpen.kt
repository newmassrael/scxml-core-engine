// SCE-GENERATED — DO NOT EDIT
// source-hash: 6d71812086a998a80412c256f457bab009adc259010083f54f89e8deddbca71a
// SCE-MAP: shape_kind_open.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.shape_kind_open

@JvmInline
@OptIn(ExperimentalUnsignedTypes::class)
value class ShapeKindOpen(val raw: UByte) {
    companion object {
        val DOT = ShapeKindOpen(0u.toUByte())
        val CIRCLE = ShapeKindOpen(1u.toUByte())
        val RECT = ShapeKindOpen(2u.toUByte())

        /** The value [raw] stands for. Total: the declared set is open. */
        fun fromUnderlying(raw: UByte) = ShapeKindOpen(raw)
    }

    /** The carrier value this holds. */
    fun toUnderlying(): UByte = raw

    /**
     * The name the document declares for this value, or null for a value
     * the open set holds without naming.
     */
    fun declaredName(): String? = when (this) {
        DOT -> "dot"
        CIRCLE -> "circle"
        RECT -> "rect"
        else -> null
    }
}
