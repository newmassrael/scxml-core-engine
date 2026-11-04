// SCE-GENERATED — DO NOT EDIT
// source-hash: e939b2700c7d391ee77811c0c5056f1bd75fa31a962245a8882cc047f3a138a0
// SCE-MAP: forge_call_level.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.forge_call_level

@OptIn(ExperimentalUnsignedTypes::class)
enum class ForgeCallLevel(val raw: UByte) {
    LIGHT(10u.toUByte()),
    HEAVY(20u.toUByte()),
    ;

    /** The carrier value this variant declares. */
    fun toUnderlying(): UByte = raw

    /** The name the document declares for this variant. */
    fun declaredName(): String = when (this) {
        LIGHT -> "light"
        HEAVY -> "heavy"
    }

    companion object {
        /**
         * The variant [raw] declares, or null — the declared set is closed,
         * so a value outside it is not a value of this type.
         */
        fun fromUnderlying(raw: UByte): ForgeCallLevel? =
            entries.firstOrNull { it.raw == raw }
    }
}
