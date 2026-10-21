// SCE-GENERATED — DO NOT EDIT
// source-hash: 09d8fec62ddcc806d17c4e1da241b802dbe4f5317b3bd415a705b2ad99d82f3e
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
