// SCE-GENERATED — DO NOT EDIT
// source-hash: f01c184986658113c3d88b7783aaf923ed4e30d14575633fec8ae3793b107abd
// SCE-MAP: rpc_status.scxml:12 :: _forge_body

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
    INTERNAL_ERROR(13u.toUByte()),
    UNAVAILABLE(14u.toUByte()),
    ;

    /** The carrier value this variant declares. */
    fun toUnderlying(): UByte = raw

    /** The name the document declares for this variant. */
    fun declaredName(): String = when (this) {
        OK -> "ok"
        CANCELLED -> "cancelled"
        INVALID_ARGUMENT -> "invalidArgument"
        DEADLINE_EXCEEDED -> "deadlineExceeded"
        NOT_FOUND -> "notFound"
        UNIMPLEMENTED -> "unimplemented"
        INTERNAL_ERROR -> "internalError"
        UNAVAILABLE -> "unavailable"
    }

    companion object {
        /**
         * The variant [raw] declares, or null — the declared set is closed,
         * so a value outside it is not a value of this type.
         */
        fun fromUnderlying(raw: UByte): RpcStatus? =
            entries.firstOrNull { it.raw == raw }
    }
}
