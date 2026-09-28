// SCE-GENERATED — DO NOT EDIT
// source-hash: c99c3c0939ed7b1256239958164f8fd91e9fc2a690644b8df67efbcf58589a94
// SCE-MAP: pattern_kind.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.pattern_kind

@OptIn(ExperimentalUnsignedTypes::class)
enum class PatternKind(val raw: UShort) {
    FIRE_FORGET(1u.toUShort()),
    RPC_REQUEST(2u.toUShort()),
    RPC_REPLY(3u.toUShort()),
    EVENT_SUBSCRIBE(4u.toUShort()),
    EVENT_UNSUBSCRIBE(5u.toUShort()),
    EVENT_NOTIFY(6u.toUShort()),
    FIELD_READ(7u.toUShort()),
    FIELD_WRITE(8u.toUShort()),
    FIELD_NOTIFY(9u.toUShort()),
    INVOKE_START(14u.toUShort()),
    INVOKE_STARTED(15u.toUShort()),
    CHILD_EVENT(16u.toUShort()),
    PARENT_EVENT(17u.toUShort()),
    INVOKE_DONE(18u.toUShort()),
    INVOKE_CANCEL(19u.toUShort()),
    INVOKE_ERROR(20u.toUShort()),
    PARALLEL_REGION_DONE(21u.toUShort()),
    ;

    /** The carrier value this variant declares. */
    fun toUnderlying(): UShort = raw

    /** The name the document declares for this variant. */
    fun declaredName(): String = when (this) {
        FIRE_FORGET -> "fireForget"
        RPC_REQUEST -> "rpcRequest"
        RPC_REPLY -> "rpcReply"
        EVENT_SUBSCRIBE -> "eventSubscribe"
        EVENT_UNSUBSCRIBE -> "eventUnsubscribe"
        EVENT_NOTIFY -> "eventNotify"
        FIELD_READ -> "fieldRead"
        FIELD_WRITE -> "fieldWrite"
        FIELD_NOTIFY -> "fieldNotify"
        INVOKE_START -> "invokeStart"
        INVOKE_STARTED -> "invokeStarted"
        CHILD_EVENT -> "childEvent"
        PARENT_EVENT -> "parentEvent"
        INVOKE_DONE -> "invokeDone"
        INVOKE_CANCEL -> "invokeCancel"
        INVOKE_ERROR -> "invokeError"
        PARALLEL_REGION_DONE -> "parallelRegionDone"
    }

    companion object {
        /**
         * The variant [raw] declares, or null — the declared set is closed,
         * so a value outside it is not a value of this type.
         */
        fun fromUnderlying(raw: UShort): PatternKind? =
            entries.firstOrNull { it.raw == raw }
    }
}
