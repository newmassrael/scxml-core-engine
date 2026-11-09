// SCE-GENERATED — DO NOT EDIT
// source-hash: 39c188440acd2a30e1c96c90e825a582fc7011a2c558c89e70ea674bb5bda40e
// SCE-MAP: forge_call_status.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.forge_call_status

@OptIn(ExperimentalUnsignedTypes::class)
enum class ForgeCallStatus(val raw: UByte) {
    OPEN(1u.toUByte()),
    HELD(2u.toUByte()),
    DONE(3u.toUByte()),
    ;

    /** The carrier value this variant declares. */
    fun toUnderlying(): UByte = raw

    /** The name the document declares for this variant. */
    fun declaredName(): String = when (this) {
        OPEN -> "open"
        HELD -> "held"
        DONE -> "done"
    }

    companion object {
        /**
         * The variant [raw] declares, or null — the declared set is closed,
         * so a value outside it is not a value of this type.
         */
        fun fromUnderlying(raw: UByte): ForgeCallStatus? =
            entries.firstOrNull { it.raw == raw }
    }
}
