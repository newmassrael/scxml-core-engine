// SCE-GENERATED — DO NOT EDIT
// source-hash: 6e3fa3ae1d1df10dd29fa4c2afbffd093df1f9518f7a44d21f2f01a956798b04
// source: causal_admission.scxml
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
