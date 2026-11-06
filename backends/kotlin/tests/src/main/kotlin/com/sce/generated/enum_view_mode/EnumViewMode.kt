// SCE-GENERATED — DO NOT EDIT
// source-hash: 9aa28cb8a6152625fb0eb1560e8ab3014da5864745e85b6f3b6e37672c04dcf6
// SCE-MAP: enum_view_mode.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.enum_view_mode

@OptIn(ExperimentalUnsignedTypes::class)
enum class EnumViewMode(val raw: UByte) {
    MONTH(0u.toUByte()),
    WEEK(1u.toUByte()),
    DAY(2u.toUByte()),
    AGENDA_LIST(3u.toUByte()),
    ;

    /** The carrier value this variant declares. */
    fun toUnderlying(): UByte = raw

    /** The name the document declares for this variant. */
    fun declaredName(): String = when (this) {
        MONTH -> "month"
        WEEK -> "week"
        DAY -> "day"
        AGENDA_LIST -> "agenda_list"
    }

    companion object {
        /**
         * The variant [raw] declares, or null — the declared set is closed,
         * so a value outside it is not a value of this type.
         */
        fun fromUnderlying(raw: UByte): EnumViewMode? =
            entries.firstOrNull { it.raw == raw }
    }
}
