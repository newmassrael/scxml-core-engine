// SCE-GENERATED — DO NOT EDIT
// source-hash: e8f320f75dfeb19d8a1c86b952355e53aa00519f4b2e9108eb558cc94a73e2ba
// source: error_reason.scxml
// SCE-MAP: error_reason.scxml:60 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.error_reason

@OptIn(ExperimentalUnsignedTypes::class)
enum class ErrorReason(val raw: UByte) {
    EXPRESSION_FAILED(1u.toUByte()),
    LOCATION_INVALID(2u.toUByte()),
    ASSIGNMENT_FAILED(3u.toUByte()),
    SCRIPT_FAILED(4u.toUByte()),
    DATA_INIT_FAILED(5u.toUByte()),
    FOREACH_INVALID(6u.toUByte()),
    SEND_TYPE_UNSUPPORTED(7u.toUByte()),
    SEND_TARGET_INVALID(8u.toUByte()),
    SEND_DELAY_INVALID(9u.toUByte()),
    PARAM_UNFIT(10u.toUByte()),
    PROCESSOR_UNREGISTERED(11u.toUByte()),
    INVOKE_TYPE_UNSUPPORTED(12u.toUByte()),
    INVOKE_SRC_NOT_FOUND(13u.toUByte()),
    INVOKE_CHILD_INIT_FAILED(14u.toUByte()),
    RESERVED_PARAM_CONFLICT(15u.toUByte()),
    SESSION_TRANSPORT_UNAVAILABLE(16u.toUByte()),
    PAYLOAD_REFUSED(17u.toUByte()),
    CAPACITY_EXCEEDED(18u.toUByte()),
    TARGET_UNREACHABLE(64u.toUByte()),
    HTTP_TARGET_INVALID(65u.toUByte()),
    TRANSPORT_UNAVAILABLE(66u.toUByte()),
    SEND_FAILED(67u.toUByte()),
    DELIVERY_EXHAUSTED(68u.toUByte()),
    ENVELOPE_CORRUPT(69u.toUByte()),
    INVOKE_CHILD_LOST(70u.toUByte()),
    PARALLEL_BARRIER_TIMEOUT(71u.toUByte()),
    DEDUP_WINDOW_OVERFLOW(72u.toUByte()),
    PEER_PARTITIONED(73u.toUByte()),
    BACKPRESSURE_DROP(74u.toUByte()),
    UNAUTHORIZED(75u.toUByte()),
    MISSING_SEQUENCE(76u.toUByte()),
    ORDERING_GAP(77u.toUByte()),
    REGION_PARTITIONED(78u.toUByte()),
    RPC_REPLY_FROM_UNDECLARED_PEER(79u.toUByte()),
    OUTBOUND_STALE_DROP(80u.toUByte()),
    NOTIFICATION_DEADLINE_MISSED(81u.toUByte()),
    RESTORED(82u.toUByte()),
    CAUSAL_VIEW_MISMATCH(83u.toUByte()),
    ;

    /** The carrier value this variant declares. */
    fun toUnderlying(): UByte = raw

    /** The name the document declares for this variant. */
    fun declaredName(): String = when (this) {
        EXPRESSION_FAILED -> "expressionFailed"
        LOCATION_INVALID -> "locationInvalid"
        ASSIGNMENT_FAILED -> "assignmentFailed"
        SCRIPT_FAILED -> "scriptFailed"
        DATA_INIT_FAILED -> "dataInitFailed"
        FOREACH_INVALID -> "foreachInvalid"
        SEND_TYPE_UNSUPPORTED -> "sendTypeUnsupported"
        SEND_TARGET_INVALID -> "sendTargetInvalid"
        SEND_DELAY_INVALID -> "sendDelayInvalid"
        PARAM_UNFIT -> "paramUnfit"
        PROCESSOR_UNREGISTERED -> "processorUnregistered"
        INVOKE_TYPE_UNSUPPORTED -> "invokeTypeUnsupported"
        INVOKE_SRC_NOT_FOUND -> "invokeSrcNotFound"
        INVOKE_CHILD_INIT_FAILED -> "invokeChildInitFailed"
        RESERVED_PARAM_CONFLICT -> "reservedParamConflict"
        SESSION_TRANSPORT_UNAVAILABLE -> "sessionTransportUnavailable"
        PAYLOAD_REFUSED -> "payloadRefused"
        CAPACITY_EXCEEDED -> "capacityExceeded"
        TARGET_UNREACHABLE -> "targetUnreachable"
        HTTP_TARGET_INVALID -> "httpTargetInvalid"
        TRANSPORT_UNAVAILABLE -> "transportUnavailable"
        SEND_FAILED -> "sendFailed"
        DELIVERY_EXHAUSTED -> "deliveryExhausted"
        ENVELOPE_CORRUPT -> "envelopeCorrupt"
        INVOKE_CHILD_LOST -> "invokeChildLost"
        PARALLEL_BARRIER_TIMEOUT -> "parallelBarrierTimeout"
        DEDUP_WINDOW_OVERFLOW -> "dedupWindowOverflow"
        PEER_PARTITIONED -> "peerPartitioned"
        BACKPRESSURE_DROP -> "backpressureDrop"
        UNAUTHORIZED -> "unauthorized"
        MISSING_SEQUENCE -> "missingSequence"
        ORDERING_GAP -> "orderingGap"
        REGION_PARTITIONED -> "regionPartitioned"
        RPC_REPLY_FROM_UNDECLARED_PEER -> "rpcReplyFromUndeclaredPeer"
        OUTBOUND_STALE_DROP -> "outboundStaleDrop"
        NOTIFICATION_DEADLINE_MISSED -> "notificationDeadlineMissed"
        RESTORED -> "restored"
        CAUSAL_VIEW_MISMATCH -> "causalViewMismatch"
    }

    companion object {
        /**
         * The variant [raw] declares, or null — the declared set is closed,
         * so a value outside it is not a value of this type.
         */
        fun fromUnderlying(raw: UByte): ErrorReason? =
            entries.firstOrNull { it.raw == raw }
    }
}
