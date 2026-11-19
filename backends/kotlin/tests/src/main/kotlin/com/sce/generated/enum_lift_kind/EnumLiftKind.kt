// SCE-GENERATED — DO NOT EDIT
// source-hash: 257fc4ff0ee49e833cae625af6f30d3cba25016aee82a12b9a36aa0f4666b33d
// source: enum_lift_kind.scxml
// SCE-MAP: enum_lift_kind.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.enum_lift_kind

@OptIn(ExperimentalUnsignedTypes::class)
enum class EnumLiftKind(val raw: UByte) {
    MEETING(0u.toUByte()),
    TASK(1u.toUByte()),
    ;

    /** The carrier value this variant declares. */
    fun toUnderlying(): UByte = raw

    /** The name the document declares for this variant. */
    fun declaredName(): String = when (this) {
        MEETING -> "meeting"
        TASK -> "task"
    }

    companion object {
        /**
         * The variant [raw] declares, or null — the declared set is closed,
         * so a value outside it is not a value of this type.
         */
        fun fromUnderlying(raw: UByte): EnumLiftKind? =
            entries.firstOrNull { it.raw == raw }
    }
}
