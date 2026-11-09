// SCE-GENERATED — DO NOT EDIT
// source-hash: f2f8682669670c7aaedbccb3102dd71d7b187eac29a32d3d20509a5bc070c12e
// SCE-MAP: causal_admission.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.causal_admission

@OptIn(ExperimentalUnsignedTypes::class)
enum class CausalAdmission(val raw: UByte) {
    DROP(0u.toUByte()),
    DELIVER(1u.toUByte()),
    HOLD(2u.toUByte()),
    ;

    /** The carrier value this variant declares. */
    fun toUnderlying(): UByte = raw

    /** The name the document declares for this variant. */
    fun declaredName(): String = when (this) {
        DROP -> "drop"
        DELIVER -> "deliver"
        HOLD -> "hold"
    }

    companion object {
        /**
         * The variant [raw] declares, or null — the declared set is closed,
         * so a value outside it is not a value of this type.
         */
        fun fromUnderlying(raw: UByte): CausalAdmission? =
            entries.firstOrNull { it.raw == raw }
    }
}
