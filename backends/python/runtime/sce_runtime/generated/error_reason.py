# SCE-GENERATED — DO NOT EDIT
# source-hash: e8f320f75dfeb19d8a1c86b952355e53aa00519f4b2e9108eb558cc94a73e2ba
# source: error_reason.scxml
# SCE-MAP: error_reason.scxml:60 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
# Runtime: none
# Underlying integer carrier: uint8
# Do not edit — regenerate from the source SCXML file.

from enum import IntEnum
from typing import Optional


class ErrorReason(IntEnum):
    EXPRESSION_FAILED = 1
    LOCATION_INVALID = 2
    ASSIGNMENT_FAILED = 3
    SCRIPT_FAILED = 4
    DATA_INIT_FAILED = 5
    FOREACH_INVALID = 6
    SEND_TYPE_UNSUPPORTED = 7
    SEND_TARGET_INVALID = 8
    SEND_DELAY_INVALID = 9
    PARAM_UNFIT = 10
    PROCESSOR_UNREGISTERED = 11
    INVOKE_TYPE_UNSUPPORTED = 12
    INVOKE_SRC_NOT_FOUND = 13
    INVOKE_CHILD_INIT_FAILED = 14
    RESERVED_PARAM_CONFLICT = 15
    SESSION_TRANSPORT_UNAVAILABLE = 16
    PAYLOAD_REFUSED = 17
    CAPACITY_EXCEEDED = 18
    TARGET_UNREACHABLE = 64
    HTTP_TARGET_INVALID = 65
    TRANSPORT_UNAVAILABLE = 66
    SEND_FAILED = 67
    DELIVERY_EXHAUSTED = 68
    ENVELOPE_CORRUPT = 69
    INVOKE_CHILD_LOST = 70
    PARALLEL_BARRIER_TIMEOUT = 71
    DEDUP_WINDOW_OVERFLOW = 72
    PEER_PARTITIONED = 73
    BACKPRESSURE_DROP = 74
    UNAUTHORIZED = 75
    MISSING_SEQUENCE = 76
    ORDERING_GAP = 77
    REGION_PARTITIONED = 78
    RPC_REPLY_FROM_UNDECLARED_PEER = 79
    OUTBOUND_STALE_DROP = 80
    NOTIFICATION_DEADLINE_MISSED = 81
    RESTORED = 82
    CAUSAL_VIEW_MISMATCH = 83

    def to_underlying(self) -> int:
        """The carrier value this holds."""
        return int(self)

    def declared_name(self) -> str:
        """The name the document declares for this variant."""
        return _ErrorReason_DECLARED[int(self)]

    @classmethod
    def from_underlying(cls, raw: int) -> Optional["ErrorReason"]:
        """
        The variant ``raw`` declares, or None — the declared set is closed,
        so a value outside it is not a value of this type.
        """
        try:
            return cls(raw)
        except ValueError:
            return None


_ErrorReason_DECLARED = {
    1: "expressionFailed",
    2: "locationInvalid",
    3: "assignmentFailed",
    4: "scriptFailed",
    5: "dataInitFailed",
    6: "foreachInvalid",
    7: "sendTypeUnsupported",
    8: "sendTargetInvalid",
    9: "sendDelayInvalid",
    10: "paramUnfit",
    11: "processorUnregistered",
    12: "invokeTypeUnsupported",
    13: "invokeSrcNotFound",
    14: "invokeChildInitFailed",
    15: "reservedParamConflict",
    16: "sessionTransportUnavailable",
    17: "payloadRefused",
    18: "capacityExceeded",
    64: "targetUnreachable",
    65: "httpTargetInvalid",
    66: "transportUnavailable",
    67: "sendFailed",
    68: "deliveryExhausted",
    69: "envelopeCorrupt",
    70: "invokeChildLost",
    71: "parallelBarrierTimeout",
    72: "dedupWindowOverflow",
    73: "peerPartitioned",
    74: "backpressureDrop",
    75: "unauthorized",
    76: "missingSequence",
    77: "orderingGap",
    78: "regionPartitioned",
    79: "rpcReplyFromUndeclaredPeer",
    80: "outboundStaleDrop",
    81: "notificationDeadlineMissed",
    82: "restored",
    83: "causalViewMismatch",
}
