// SCE-GENERATED — DO NOT EDIT
// source-hash: f6148b213f9ed141b381c5b54aabc8b1a585f1b96413497e91bf4f50d5cda1ce
// SCE-MAP: payload_codec.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_PAYLOAD_CODEC_H
#define SCE_FORGE_PAYLOAD_CODEC_H

#include <cstdint>
#include <optional>

namespace SCE::Generated::PayloadCodec {

enum class PayloadCodec : uint8_t {
    None = 0,
    Json = 1,
    Cbor = 2,
    Typed = 3,
    Raw = 4,
};

/// The carrier value this holds.
constexpr uint8_t to_underlying(PayloadCodec value) noexcept {
    return static_cast<uint8_t>(value);
}

/// The variant `raw` declares, or `std::nullopt` — the declared set is
/// closed, so a value outside it is not a value of this type.
constexpr std::optional<PayloadCodec> from_underlying(uint8_t raw) noexcept {
    switch (raw) {
    case 0:
        return PayloadCodec::None;
    case 1:
        return PayloadCodec::Json;
    case 2:
        return PayloadCodec::Cbor;
    case 3:
        return PayloadCodec::Typed;
    case 4:
        return PayloadCodec::Raw;
    default:
        return std::nullopt;
    }
}

}  // namespace SCE::Generated::PayloadCodec

#endif  // SCE_FORGE_PAYLOAD_CODEC_H
