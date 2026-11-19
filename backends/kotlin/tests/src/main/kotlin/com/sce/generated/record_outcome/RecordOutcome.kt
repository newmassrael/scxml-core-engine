// SCE-GENERATED — DO NOT EDIT
// source-hash: ba163df4f9edf81a451d27f567c416db19629a5e4279555ff80cf4e47f57b6b0
// source: sce:std/shared/record_outcome.scxml
// SCE-MAP: record_outcome.scxml:22 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.record_outcome

@OptIn(ExperimentalUnsignedTypes::class)
enum class RecordOutcome(val raw: UByte) {
    NONE(0u.toUByte()),
    ACCEPTED(1u.toUByte()),
    HELD(2u.toUByte()),
    WAITING(3u.toUByte()),
    REFUSED(4u.toUByte()),
    ;

    /** The carrier value this variant declares. */
    fun toUnderlying(): UByte = raw

    /** The name the document declares for this variant. */
    fun declaredName(): String = when (this) {
        NONE -> "none"
        ACCEPTED -> "accepted"
        HELD -> "held"
        WAITING -> "waiting"
        REFUSED -> "refused"
    }

    companion object {
        /**
         * The variant [raw] declares, or null — the declared set is closed,
         * so a value outside it is not a value of this type.
         */
        fun fromUnderlying(raw: UByte): RecordOutcome? =
            entries.firstOrNull { it.raw == raw }
    }
}
