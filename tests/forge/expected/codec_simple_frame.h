// SCE-MAP: codec_simple_frame:3

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_SIMPLE_FRAME_H
#define SCE_FORGE_CODEC_SIMPLE_FRAME_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecSimpleFrame {

struct CodecSimpleFrame {
    uint8_t msgId;
    uint8_t length;
    uint16_t payload;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecSimpleFrame> decode(::SCE::Forge::SceCursor& cursor) {
        const std::uint8_t* raw = cursor.peek_slice(4);
        if (raw == nullptr) return std::nullopt;
        CodecSimpleFrame value{
            .msgId = raw[0],
            .length = raw[1],
            .payload = static_cast<uint16_t>((static_cast<uint16_t>(raw[2]) << 8) | raw[3]),
        };
        if (!cursor.advance(4)) return std::nullopt;
        return value;
    }

    std::vector<uint8_t> encode() const {
        return {
            msgId,
            length,
            static_cast<uint8_t>((payload >> 8) & 0xFF),
            static_cast<uint8_t>(payload & 0xFF)
        };
    }
};

}  // namespace SCE::Generated::CodecSimpleFrame

#endif  // SCE_FORGE_CODEC_SIMPLE_FRAME_H
