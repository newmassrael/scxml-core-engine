// SCE-GENERATED — DO NOT EDIT
// source-hash: 58f5a17e479c571b0f1a95c9ced6b8f03b5d957325be03d16f15d59d99727b1b
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
