// SCE-GENERATED — DO NOT EDIT
// source-hash: be25ee8255a32c7ec240ba31f48b3e3888b421310628dd926f72cb5a0da29c66
// source: grow_kind.scxml
// SCE-MAP: grow_kind.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.grow_kind

@OptIn(ExperimentalUnsignedTypes::class)
enum class GrowKind(val raw: UByte) {
    SINGLE(1u.toUByte()),
    NONE(2u.toUByte()),
    ;

    /** The carrier value this variant declares. */
    fun toUnderlying(): UByte = raw

    /** The name the document declares for this variant. */
    fun declaredName(): String = when (this) {
        SINGLE -> "single"
        NONE -> "none"
    }

    companion object {
        /**
         * The variant [raw] declares, or null — the declared set is closed,
         * so a value outside it is not a value of this type.
         */
        fun fromUnderlying(raw: UByte): GrowKind? =
            entries.firstOrNull { it.raw == raw }
    }
}
