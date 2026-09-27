// SCE-GENERATED — DO NOT EDIT
// source-hash: 0ffd4f5aaee672eb45b262966e33f5e18902327fd6170e44dc26c067a1ddada4
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
