// SCE-GENERATED — DO NOT EDIT
// source-hash: c99c3c0939ed7b1256239958164f8fd91e9fc2a690644b8df67efbcf58589a94
// SCE-MAP: pattern_kind.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_PATTERN_KIND_H
#define SCE_FORGE_PATTERN_KIND_H

#include <cstdint>
#include <optional>

namespace SCE::Generated::PatternKind {

enum class PatternKind : uint16_t {
    FireForget = 1,
    RpcRequest = 2,
    RpcReply = 3,
    EventSubscribe = 4,
    EventUnsubscribe = 5,
    EventNotify = 6,
    FieldRead = 7,
    FieldWrite = 8,
    FieldNotify = 9,
    InvokeStart = 14,
    InvokeStarted = 15,
    ChildEvent = 16,
    ParentEvent = 17,
    InvokeDone = 18,
    InvokeCancel = 19,
    InvokeError = 20,
    ParallelRegionDone = 21,
};

/// The carrier value this holds.
constexpr uint16_t to_underlying(PatternKind value) noexcept {
    return static_cast<uint16_t>(value);
}

/// The name the document declares for this value, or `nullptr` for a value
/// no variant names — one an `enum class` can hold whether or not its set is
/// open, since it holds every carrier value.
constexpr const char *declared_name(PatternKind value) noexcept {
    switch (value) {
    case PatternKind::FireForget:
        return "fireForget";
    case PatternKind::RpcRequest:
        return "rpcRequest";
    case PatternKind::RpcReply:
        return "rpcReply";
    case PatternKind::EventSubscribe:
        return "eventSubscribe";
    case PatternKind::EventUnsubscribe:
        return "eventUnsubscribe";
    case PatternKind::EventNotify:
        return "eventNotify";
    case PatternKind::FieldRead:
        return "fieldRead";
    case PatternKind::FieldWrite:
        return "fieldWrite";
    case PatternKind::FieldNotify:
        return "fieldNotify";
    case PatternKind::InvokeStart:
        return "invokeStart";
    case PatternKind::InvokeStarted:
        return "invokeStarted";
    case PatternKind::ChildEvent:
        return "childEvent";
    case PatternKind::ParentEvent:
        return "parentEvent";
    case PatternKind::InvokeDone:
        return "invokeDone";
    case PatternKind::InvokeCancel:
        return "invokeCancel";
    case PatternKind::InvokeError:
        return "invokeError";
    case PatternKind::ParallelRegionDone:
        return "parallelRegionDone";
    }
    return nullptr;
}

/// The variant `raw` declares, or `std::nullopt` — the declared set is
/// closed, so a value outside it is not a value of this type.
constexpr std::optional<PatternKind> from_underlying(uint16_t raw) noexcept {
    switch (raw) {
    case 1:
        return PatternKind::FireForget;
    case 2:
        return PatternKind::RpcRequest;
    case 3:
        return PatternKind::RpcReply;
    case 4:
        return PatternKind::EventSubscribe;
    case 5:
        return PatternKind::EventUnsubscribe;
    case 6:
        return PatternKind::EventNotify;
    case 7:
        return PatternKind::FieldRead;
    case 8:
        return PatternKind::FieldWrite;
    case 9:
        return PatternKind::FieldNotify;
    case 14:
        return PatternKind::InvokeStart;
    case 15:
        return PatternKind::InvokeStarted;
    case 16:
        return PatternKind::ChildEvent;
    case 17:
        return PatternKind::ParentEvent;
    case 18:
        return PatternKind::InvokeDone;
    case 19:
        return PatternKind::InvokeCancel;
    case 20:
        return PatternKind::InvokeError;
    case 21:
        return PatternKind::ParallelRegionDone;
    default:
        return std::nullopt;
    }
}

}  // namespace SCE::Generated::PatternKind

#endif  // SCE_FORGE_PATTERN_KIND_H
