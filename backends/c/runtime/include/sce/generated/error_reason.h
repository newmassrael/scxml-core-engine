// SCE-GENERATED — DO NOT EDIT
// source-hash: 23b734cba3b99db67184128a8a40e156adea520255d96934f32e5bd6eef884be
// source: error_reason.scxml
// SCE-MAP: error_reason.scxml:60 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum") */
/* Runtime: none */
/* Underlying integer carrier: uint8_t */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_ERROR_REASON_H
#define SCE_FORGE_ERROR_REASON_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#ifndef SCE_ENUM_ERROR_REASON_DECLARED
#define SCE_ENUM_ERROR_REASON_DECLARED
typedef enum {
    ERROR_REASON_EXPRESSION_FAILED = 1,
    ERROR_REASON_LOCATION_INVALID = 2,
    ERROR_REASON_ASSIGNMENT_FAILED = 3,
    ERROR_REASON_SCRIPT_FAILED = 4,
    ERROR_REASON_DATA_INIT_FAILED = 5,
    ERROR_REASON_FOREACH_INVALID = 6,
    ERROR_REASON_SEND_TYPE_UNSUPPORTED = 7,
    ERROR_REASON_SEND_TARGET_INVALID = 8,
    ERROR_REASON_SEND_DELAY_INVALID = 9,
    ERROR_REASON_PARAM_UNFIT = 10,
    ERROR_REASON_PROCESSOR_UNREGISTERED = 11,
    ERROR_REASON_INVOKE_TYPE_UNSUPPORTED = 12,
    ERROR_REASON_INVOKE_SRC_NOT_FOUND = 13,
    ERROR_REASON_INVOKE_CHILD_INIT_FAILED = 14,
    ERROR_REASON_RESERVED_PARAM_CONFLICT = 15,
    ERROR_REASON_SESSION_TRANSPORT_UNAVAILABLE = 16,
    ERROR_REASON_PAYLOAD_REFUSED = 17,
    ERROR_REASON_CAPACITY_EXCEEDED = 18,
    ERROR_REASON_TARGET_UNREACHABLE = 64,
    ERROR_REASON_HTTP_TARGET_INVALID = 65,
    ERROR_REASON_TRANSPORT_UNAVAILABLE = 66,
    ERROR_REASON_SEND_FAILED = 67,
    ERROR_REASON_DELIVERY_EXHAUSTED = 68,
    ERROR_REASON_ENVELOPE_CORRUPT = 69,
    ERROR_REASON_INVOKE_CHILD_LOST = 70,
    ERROR_REASON_PARALLEL_BARRIER_TIMEOUT = 71,
    ERROR_REASON_DEDUP_WINDOW_OVERFLOW = 72,
    ERROR_REASON_PEER_PARTITIONED = 73,
    ERROR_REASON_BACKPRESSURE_DROP = 74,
    ERROR_REASON_UNAUTHORIZED = 75,
    ERROR_REASON_MISSING_SEQUENCE = 76,
    ERROR_REASON_ORDERING_GAP = 77,
    ERROR_REASON_REGION_PARTITIONED = 78,
    ERROR_REASON_RPC_REPLY_FROM_UNDECLARED_PEER = 79,
    ERROR_REASON_OUTBOUND_STALE_DROP = 80,
    ERROR_REASON_NOTIFICATION_DEADLINE_MISSED = 81,
    ERROR_REASON_RESTORED = 82,
    ERROR_REASON_CAUSAL_VIEW_MISMATCH = 83,
    ERROR_REASON_RESPONSE_UNAUTHENTICATED = 84,
    ERROR_REASON_OUTCOME_UNKNOWN = 85,
} ErrorReason_t;

/* The name the document declares for this value, or NULL for a value no
 * variant names. */
static inline const char *error_reason_declared_name(ErrorReason_t value) {
    switch (value) {
    case ERROR_REASON_EXPRESSION_FAILED:
        return "expressionFailed";
    case ERROR_REASON_LOCATION_INVALID:
        return "locationInvalid";
    case ERROR_REASON_ASSIGNMENT_FAILED:
        return "assignmentFailed";
    case ERROR_REASON_SCRIPT_FAILED:
        return "scriptFailed";
    case ERROR_REASON_DATA_INIT_FAILED:
        return "dataInitFailed";
    case ERROR_REASON_FOREACH_INVALID:
        return "foreachInvalid";
    case ERROR_REASON_SEND_TYPE_UNSUPPORTED:
        return "sendTypeUnsupported";
    case ERROR_REASON_SEND_TARGET_INVALID:
        return "sendTargetInvalid";
    case ERROR_REASON_SEND_DELAY_INVALID:
        return "sendDelayInvalid";
    case ERROR_REASON_PARAM_UNFIT:
        return "paramUnfit";
    case ERROR_REASON_PROCESSOR_UNREGISTERED:
        return "processorUnregistered";
    case ERROR_REASON_INVOKE_TYPE_UNSUPPORTED:
        return "invokeTypeUnsupported";
    case ERROR_REASON_INVOKE_SRC_NOT_FOUND:
        return "invokeSrcNotFound";
    case ERROR_REASON_INVOKE_CHILD_INIT_FAILED:
        return "invokeChildInitFailed";
    case ERROR_REASON_RESERVED_PARAM_CONFLICT:
        return "reservedParamConflict";
    case ERROR_REASON_SESSION_TRANSPORT_UNAVAILABLE:
        return "sessionTransportUnavailable";
    case ERROR_REASON_PAYLOAD_REFUSED:
        return "payloadRefused";
    case ERROR_REASON_CAPACITY_EXCEEDED:
        return "capacityExceeded";
    case ERROR_REASON_TARGET_UNREACHABLE:
        return "targetUnreachable";
    case ERROR_REASON_HTTP_TARGET_INVALID:
        return "httpTargetInvalid";
    case ERROR_REASON_TRANSPORT_UNAVAILABLE:
        return "transportUnavailable";
    case ERROR_REASON_SEND_FAILED:
        return "sendFailed";
    case ERROR_REASON_DELIVERY_EXHAUSTED:
        return "deliveryExhausted";
    case ERROR_REASON_ENVELOPE_CORRUPT:
        return "envelopeCorrupt";
    case ERROR_REASON_INVOKE_CHILD_LOST:
        return "invokeChildLost";
    case ERROR_REASON_PARALLEL_BARRIER_TIMEOUT:
        return "parallelBarrierTimeout";
    case ERROR_REASON_DEDUP_WINDOW_OVERFLOW:
        return "dedupWindowOverflow";
    case ERROR_REASON_PEER_PARTITIONED:
        return "peerPartitioned";
    case ERROR_REASON_BACKPRESSURE_DROP:
        return "backpressureDrop";
    case ERROR_REASON_UNAUTHORIZED:
        return "unauthorized";
    case ERROR_REASON_MISSING_SEQUENCE:
        return "missingSequence";
    case ERROR_REASON_ORDERING_GAP:
        return "orderingGap";
    case ERROR_REASON_REGION_PARTITIONED:
        return "regionPartitioned";
    case ERROR_REASON_RPC_REPLY_FROM_UNDECLARED_PEER:
        return "rpcReplyFromUndeclaredPeer";
    case ERROR_REASON_OUTBOUND_STALE_DROP:
        return "outboundStaleDrop";
    case ERROR_REASON_NOTIFICATION_DEADLINE_MISSED:
        return "notificationDeadlineMissed";
    case ERROR_REASON_RESTORED:
        return "restored";
    case ERROR_REASON_CAUSAL_VIEW_MISMATCH:
        return "causalViewMismatch";
    case ERROR_REASON_RESPONSE_UNAUTHENTICATED:
        return "responseUnauthenticated";
    case ERROR_REASON_OUTCOME_UNKNOWN:
        return "outcomeUnknown";
    }
    return NULL;
}
#endif  /* SCE_ENUM_ERROR_REASON_DECLARED */

/* The carrier value this holds. */
static inline uint8_t error_reason_to_underlying(ErrorReason_t value) {
    return (uint8_t)value;
}

/* The variant `raw` declares, written through `out`; false when `raw`
 * declares none — the declared set is closed, so a value outside it is not
 * a value of this type. */
static inline bool error_reason_from_underlying(uint8_t raw, ErrorReason_t *out) {
    switch (raw) {
    case 1:
        *out = ERROR_REASON_EXPRESSION_FAILED;
        return true;
    case 2:
        *out = ERROR_REASON_LOCATION_INVALID;
        return true;
    case 3:
        *out = ERROR_REASON_ASSIGNMENT_FAILED;
        return true;
    case 4:
        *out = ERROR_REASON_SCRIPT_FAILED;
        return true;
    case 5:
        *out = ERROR_REASON_DATA_INIT_FAILED;
        return true;
    case 6:
        *out = ERROR_REASON_FOREACH_INVALID;
        return true;
    case 7:
        *out = ERROR_REASON_SEND_TYPE_UNSUPPORTED;
        return true;
    case 8:
        *out = ERROR_REASON_SEND_TARGET_INVALID;
        return true;
    case 9:
        *out = ERROR_REASON_SEND_DELAY_INVALID;
        return true;
    case 10:
        *out = ERROR_REASON_PARAM_UNFIT;
        return true;
    case 11:
        *out = ERROR_REASON_PROCESSOR_UNREGISTERED;
        return true;
    case 12:
        *out = ERROR_REASON_INVOKE_TYPE_UNSUPPORTED;
        return true;
    case 13:
        *out = ERROR_REASON_INVOKE_SRC_NOT_FOUND;
        return true;
    case 14:
        *out = ERROR_REASON_INVOKE_CHILD_INIT_FAILED;
        return true;
    case 15:
        *out = ERROR_REASON_RESERVED_PARAM_CONFLICT;
        return true;
    case 16:
        *out = ERROR_REASON_SESSION_TRANSPORT_UNAVAILABLE;
        return true;
    case 17:
        *out = ERROR_REASON_PAYLOAD_REFUSED;
        return true;
    case 18:
        *out = ERROR_REASON_CAPACITY_EXCEEDED;
        return true;
    case 64:
        *out = ERROR_REASON_TARGET_UNREACHABLE;
        return true;
    case 65:
        *out = ERROR_REASON_HTTP_TARGET_INVALID;
        return true;
    case 66:
        *out = ERROR_REASON_TRANSPORT_UNAVAILABLE;
        return true;
    case 67:
        *out = ERROR_REASON_SEND_FAILED;
        return true;
    case 68:
        *out = ERROR_REASON_DELIVERY_EXHAUSTED;
        return true;
    case 69:
        *out = ERROR_REASON_ENVELOPE_CORRUPT;
        return true;
    case 70:
        *out = ERROR_REASON_INVOKE_CHILD_LOST;
        return true;
    case 71:
        *out = ERROR_REASON_PARALLEL_BARRIER_TIMEOUT;
        return true;
    case 72:
        *out = ERROR_REASON_DEDUP_WINDOW_OVERFLOW;
        return true;
    case 73:
        *out = ERROR_REASON_PEER_PARTITIONED;
        return true;
    case 74:
        *out = ERROR_REASON_BACKPRESSURE_DROP;
        return true;
    case 75:
        *out = ERROR_REASON_UNAUTHORIZED;
        return true;
    case 76:
        *out = ERROR_REASON_MISSING_SEQUENCE;
        return true;
    case 77:
        *out = ERROR_REASON_ORDERING_GAP;
        return true;
    case 78:
        *out = ERROR_REASON_REGION_PARTITIONED;
        return true;
    case 79:
        *out = ERROR_REASON_RPC_REPLY_FROM_UNDECLARED_PEER;
        return true;
    case 80:
        *out = ERROR_REASON_OUTBOUND_STALE_DROP;
        return true;
    case 81:
        *out = ERROR_REASON_NOTIFICATION_DEADLINE_MISSED;
        return true;
    case 82:
        *out = ERROR_REASON_RESTORED;
        return true;
    case 83:
        *out = ERROR_REASON_CAUSAL_VIEW_MISMATCH;
        return true;
    case 84:
        *out = ERROR_REASON_RESPONSE_UNAUTHENTICATED;
        return true;
    case 85:
        *out = ERROR_REASON_OUTCOME_UNKNOWN;
        return true;
    default:
        return false;
    }
}

#endif  /* SCE_FORGE_ERROR_REASON_H */
