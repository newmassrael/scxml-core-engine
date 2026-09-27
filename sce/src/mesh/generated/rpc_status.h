// SCE-GENERATED — DO NOT EDIT
// source-hash: ab119d19c373fb9e83e74bd30f74186a9e2f87ef70ba045b5c5ab8bb9e9d1849
// SCE-MAP: rpc_status.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_RPC_STATUS_H
#define SCE_FORGE_RPC_STATUS_H

#include <cstdint>
#include <optional>

namespace SCE::Generated::RpcStatus {

enum class RpcStatus : uint8_t {
    Ok = 0,
    Cancelled = 1,
    InvalidArgument = 3,
    DeadlineExceeded = 4,
    NotFound = 5,
    Unimplemented = 12,
    Internal = 13,
    Unavailable = 14,
};

/// The carrier value this holds.
constexpr uint8_t to_underlying(RpcStatus value) noexcept {
    return static_cast<uint8_t>(value);
}

/// The variant `raw` declares, or `std::nullopt` — the declared set is
/// closed, so a value outside it is not a value of this type.
constexpr std::optional<RpcStatus> from_underlying(uint8_t raw) noexcept {
    switch (raw) {
    case 0:
        return RpcStatus::Ok;
    case 1:
        return RpcStatus::Cancelled;
    case 3:
        return RpcStatus::InvalidArgument;
    case 4:
        return RpcStatus::DeadlineExceeded;
    case 5:
        return RpcStatus::NotFound;
    case 12:
        return RpcStatus::Unimplemented;
    case 13:
        return RpcStatus::Internal;
    case 14:
        return RpcStatus::Unavailable;
    default:
        return std::nullopt;
    }
}

}  // namespace SCE::Generated::RpcStatus

#endif  // SCE_FORGE_RPC_STATUS_H
