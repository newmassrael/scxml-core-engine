// SCE-GENERATED — DO NOT EDIT
// source-hash: c2e56e3837f180f07a5c044369108d947c57fa7c065dd063d76bf9f37c590bee
// SCE-MAP: enum_item_status.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.enum_item_status

@OptIn(ExperimentalUnsignedTypes::class)
enum class EnumItemStatus(val raw: UByte) {
    OPEN(2u.toUByte()),
    IN_PROGRESS(5u.toUByte()),
    DONE(7u.toUByte()),
    ;

    /** The carrier value this variant declares. */
    fun toUnderlying(): UByte = raw

    /** The name the document declares for this variant. */
    fun declaredName(): String = when (this) {
        OPEN -> "open"
        IN_PROGRESS -> "in_progress"
        DONE -> "done"
    }

    companion object {
        /**
         * The variant [raw] declares, or null — the declared set is closed,
         * so a value outside it is not a value of this type.
         */
        fun fromUnderlying(raw: UByte): EnumItemStatus? =
            entries.firstOrNull { it.raw == raw }
    }
}
