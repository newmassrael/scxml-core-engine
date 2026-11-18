// SCE-GENERATED — DO NOT EDIT
// source-hash: 687f4c0ae3532ecabd8fec6c87388a5c95d9c36571dc803cf9f1b82e7d19a7ab
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
