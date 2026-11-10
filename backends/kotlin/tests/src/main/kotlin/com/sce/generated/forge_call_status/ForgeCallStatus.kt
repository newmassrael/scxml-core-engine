// SCE-GENERATED — DO NOT EDIT
// source-hash: 81a3de342b33c3189f4a5813894163bc1e16bfc77aa7ead74e709e8ba7b570b9
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
