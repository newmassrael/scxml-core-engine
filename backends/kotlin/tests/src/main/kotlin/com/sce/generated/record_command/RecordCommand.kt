// SCE-GENERATED — DO NOT EDIT
// source-hash: d33807f273908bcf0c618164c9358f1caa01fcae9ec6f6c74812a2887ca07eec
// source: sce:std/shared/record_command.scxml
// SCE-MAP: record_command.scxml:19 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.record_command

@OptIn(ExperimentalUnsignedTypes::class)
enum class RecordCommand(val raw: UByte) {
    EDIT(0u.toUByte()),
    CLEAR(1u.toUByte()),
    RETIRE(2u.toUByte()),
    ;

    /** The carrier value this variant declares. */
    fun toUnderlying(): UByte = raw

    /** The name the document declares for this variant. */
    fun declaredName(): String = when (this) {
        EDIT -> "edit"
        CLEAR -> "clear"
        RETIRE -> "retire"
    }

    companion object {
        /**
         * The variant [raw] declares, or null — the declared set is closed,
         * so a value outside it is not a value of this type.
         */
        fun fromUnderlying(raw: UByte): RecordCommand? =
            entries.firstOrNull { it.raw == raw }
    }
}
