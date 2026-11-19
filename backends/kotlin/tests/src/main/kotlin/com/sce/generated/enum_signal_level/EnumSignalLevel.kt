// SCE-GENERATED — DO NOT EDIT
// source-hash: 18928854374741857115357dfd562d36b7bfe0eb18c6ae5bab5a940a32fc8f71
// source: enum_signal_level.scxml
// SCE-MAP: enum_signal_level.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.enum_signal_level

@OptIn(ExperimentalUnsignedTypes::class)
enum class EnumSignalLevel(val raw: UByte) {
    LOW(0u.toUByte()),
    HIGH(1u.toUByte()),
    CRITICAL(2u.toUByte()),
    ;

    /** The carrier value this variant declares. */
    fun toUnderlying(): UByte = raw

    /** The name the document declares for this variant. */
    fun declaredName(): String = when (this) {
        LOW -> "low"
        HIGH -> "high"
        CRITICAL -> "critical"
    }

    companion object {
        /**
         * The variant [raw] declares, or null — the declared set is closed,
         * so a value outside it is not a value of this type.
         */
        fun fromUnderlying(raw: UByte): EnumSignalLevel? =
            entries.firstOrNull { it.raw == raw }
    }
}
