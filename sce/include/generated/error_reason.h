// SCE-GENERATED — DO NOT EDIT
// source-hash: e8f320f75dfeb19d8a1c86b952355e53aa00519f4b2e9108eb558cc94a73e2ba
// source: error_reason.scxml
// SCE-MAP: error_reason.scxml:60 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_ERROR_REASON_H
#define SCE_FORGE_ERROR_REASON_H

#include <cstdint>
#include <optional>

namespace SCE::Generated::ErrorReason {

enum class ErrorReason : uint8_t {
    ExpressionFailed = 1,
    LocationInvalid = 2,
    AssignmentFailed = 3,
    ScriptFailed = 4,
    DataInitFailed = 5,
    ForeachInvalid = 6,
    SendTypeUnsupported = 7,
    SendTargetInvalid = 8,
    SendDelayInvalid = 9,
    ParamUnfit = 10,
    ProcessorUnregistered = 11,
    InvokeTypeUnsupported = 12,
    InvokeSrcNotFound = 13,
    InvokeChildInitFailed = 14,
    ReservedParamConflict = 15,
    SessionTransportUnavailable = 16,
    PayloadRefused = 17,
    CapacityExceeded = 18,
    TargetUnreachable = 64,
    HttpTargetInvalid = 65,
    TransportUnavailable = 66,
    SendFailed = 67,
    DeliveryExhausted = 68,
    EnvelopeCorrupt = 69,
    InvokeChildLost = 70,
    ParallelBarrierTimeout = 71,
    DedupWindowOverflow = 72,
    PeerPartitioned = 73,
    BackpressureDrop = 74,
    Unauthorized = 75,
    MissingSequence = 76,
    OrderingGap = 77,
    RegionPartitioned = 78,
    RpcReplyFromUndeclaredPeer = 79,
    OutboundStaleDrop = 80,
    NotificationDeadlineMissed = 81,
    Restored = 82,
    CausalViewMismatch = 83,
};

/// The carrier value this holds.
constexpr uint8_t to_underlying(ErrorReason value) noexcept {
    return static_cast<uint8_t>(value);
}

/// The name the document declares for this value, or `nullptr` for a value
/// no variant names — one an `enum class` can hold whether or not its set is
/// open, since it holds every carrier value.
constexpr const char *declared_name(ErrorReason value) noexcept {
    switch (value) {
    case ErrorReason::ExpressionFailed:
        return "expressionFailed";
    case ErrorReason::LocationInvalid:
        return "locationInvalid";
    case ErrorReason::AssignmentFailed:
        return "assignmentFailed";
    case ErrorReason::ScriptFailed:
        return "scriptFailed";
    case ErrorReason::DataInitFailed:
        return "dataInitFailed";
    case ErrorReason::ForeachInvalid:
        return "foreachInvalid";
    case ErrorReason::SendTypeUnsupported:
        return "sendTypeUnsupported";
    case ErrorReason::SendTargetInvalid:
        return "sendTargetInvalid";
    case ErrorReason::SendDelayInvalid:
        return "sendDelayInvalid";
    case ErrorReason::ParamUnfit:
        return "paramUnfit";
    case ErrorReason::ProcessorUnregistered:
        return "processorUnregistered";
    case ErrorReason::InvokeTypeUnsupported:
        return "invokeTypeUnsupported";
    case ErrorReason::InvokeSrcNotFound:
        return "invokeSrcNotFound";
    case ErrorReason::InvokeChildInitFailed:
        return "invokeChildInitFailed";
    case ErrorReason::ReservedParamConflict:
        return "reservedParamConflict";
    case ErrorReason::SessionTransportUnavailable:
        return "sessionTransportUnavailable";
    case ErrorReason::PayloadRefused:
        return "payloadRefused";
    case ErrorReason::CapacityExceeded:
        return "capacityExceeded";
    case ErrorReason::TargetUnreachable:
        return "targetUnreachable";
    case ErrorReason::HttpTargetInvalid:
        return "httpTargetInvalid";
    case ErrorReason::TransportUnavailable:
        return "transportUnavailable";
    case ErrorReason::SendFailed:
        return "sendFailed";
    case ErrorReason::DeliveryExhausted:
        return "deliveryExhausted";
    case ErrorReason::EnvelopeCorrupt:
        return "envelopeCorrupt";
    case ErrorReason::InvokeChildLost:
        return "invokeChildLost";
    case ErrorReason::ParallelBarrierTimeout:
        return "parallelBarrierTimeout";
    case ErrorReason::DedupWindowOverflow:
        return "dedupWindowOverflow";
    case ErrorReason::PeerPartitioned:
        return "peerPartitioned";
    case ErrorReason::BackpressureDrop:
        return "backpressureDrop";
    case ErrorReason::Unauthorized:
        return "unauthorized";
    case ErrorReason::MissingSequence:
        return "missingSequence";
    case ErrorReason::OrderingGap:
        return "orderingGap";
    case ErrorReason::RegionPartitioned:
        return "regionPartitioned";
    case ErrorReason::RpcReplyFromUndeclaredPeer:
        return "rpcReplyFromUndeclaredPeer";
    case ErrorReason::OutboundStaleDrop:
        return "outboundStaleDrop";
    case ErrorReason::NotificationDeadlineMissed:
        return "notificationDeadlineMissed";
    case ErrorReason::Restored:
        return "restored";
    case ErrorReason::CausalViewMismatch:
        return "causalViewMismatch";
    }
    return nullptr;
}

/// The variant `raw` declares, or `std::nullopt` — the declared set is
/// closed, so a value outside it is not a value of this type.
constexpr std::optional<ErrorReason> from_underlying(uint8_t raw) noexcept {
    switch (raw) {
    case 1:
        return ErrorReason::ExpressionFailed;
    case 2:
        return ErrorReason::LocationInvalid;
    case 3:
        return ErrorReason::AssignmentFailed;
    case 4:
        return ErrorReason::ScriptFailed;
    case 5:
        return ErrorReason::DataInitFailed;
    case 6:
        return ErrorReason::ForeachInvalid;
    case 7:
        return ErrorReason::SendTypeUnsupported;
    case 8:
        return ErrorReason::SendTargetInvalid;
    case 9:
        return ErrorReason::SendDelayInvalid;
    case 10:
        return ErrorReason::ParamUnfit;
    case 11:
        return ErrorReason::ProcessorUnregistered;
    case 12:
        return ErrorReason::InvokeTypeUnsupported;
    case 13:
        return ErrorReason::InvokeSrcNotFound;
    case 14:
        return ErrorReason::InvokeChildInitFailed;
    case 15:
        return ErrorReason::ReservedParamConflict;
    case 16:
        return ErrorReason::SessionTransportUnavailable;
    case 17:
        return ErrorReason::PayloadRefused;
    case 18:
        return ErrorReason::CapacityExceeded;
    case 64:
        return ErrorReason::TargetUnreachable;
    case 65:
        return ErrorReason::HttpTargetInvalid;
    case 66:
        return ErrorReason::TransportUnavailable;
    case 67:
        return ErrorReason::SendFailed;
    case 68:
        return ErrorReason::DeliveryExhausted;
    case 69:
        return ErrorReason::EnvelopeCorrupt;
    case 70:
        return ErrorReason::InvokeChildLost;
    case 71:
        return ErrorReason::ParallelBarrierTimeout;
    case 72:
        return ErrorReason::DedupWindowOverflow;
    case 73:
        return ErrorReason::PeerPartitioned;
    case 74:
        return ErrorReason::BackpressureDrop;
    case 75:
        return ErrorReason::Unauthorized;
    case 76:
        return ErrorReason::MissingSequence;
    case 77:
        return ErrorReason::OrderingGap;
    case 78:
        return ErrorReason::RegionPartitioned;
    case 79:
        return ErrorReason::RpcReplyFromUndeclaredPeer;
    case 80:
        return ErrorReason::OutboundStaleDrop;
    case 81:
        return ErrorReason::NotificationDeadlineMissed;
    case 82:
        return ErrorReason::Restored;
    case 83:
        return ErrorReason::CausalViewMismatch;
    default:
        return std::nullopt;
    }
}

}  // namespace SCE::Generated::ErrorReason

#endif  // SCE_FORGE_ERROR_REASON_H
