// SCE-GENERATED — DO NOT EDIT
// source-hash: c99c3c0939ed7b1256239958164f8fd91e9fc2a690644b8df67efbcf58589a94
// SCE-MAP: rpc_status.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.rpc_status

@OptIn(ExperimentalUnsignedTypes::class)
enum class RpcStatus(val raw: UByte) {
    OK(0u.toUByte()),
    CANCELLED(1u.toUByte()),
    INVALID_ARGUMENT(3u.toUByte()),
    DEADLINE_EXCEEDED(4u.toUByte()),
    NOT_FOUND(5u.toUByte()),
    UNIMPLEMENTED(12u.toUByte()),
    INTERNAL(13u.toUByte()),
    UNAVAILABLE(14u.toUByte()),
    ;

    /** The carrier value this variant declares. */
    fun toUnderlying(): UByte = raw

    companion object {
        /**
         * The variant [raw] declares, or null — the declared set is closed,
         * so a value outside it is not a value of this type.
         */
        fun fromUnderlying(raw: UByte): RpcStatus? =
            entries.firstOrNull { it.raw == raw }
    }
}
