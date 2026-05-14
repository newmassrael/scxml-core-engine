// SCE-MAP: codec_subbyte:3

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_SUBBYTE_H
#define SCE_FORGE_CODEC_SUBBYTE_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecSubbyte {

struct CodecSubbyte {
    uint8_t priority;
    uint8_t channel;
    uint8_t direction;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecSubbyte> decode(::SCE::Forge::SceCursor& cursor) {
        const std::uint8_t* raw = cursor.peek_slice(1);
        if (raw == nullptr) return std::nullopt;
        CodecSubbyte value{
            .priority = static_cast<uint8_t>((raw[0] >> 5) & 0x07),
            .channel = static_cast<uint8_t>((raw[0] >> 2) & 0x07),
            .direction = static_cast<uint8_t>((raw[0] >> 0) & 0x03),
        };
        if (!cursor.advance(1)) return std::nullopt;
        return value;
    }

    std::vector<uint8_t> encode() const {
        return {
            static_cast<uint8_t>(((priority & 0x07) << 5) | ((channel & 0x07) << 2) | ((direction & 0x03) << 0))
        };
    }
};

}  // namespace SCE::Generated::CodecSubbyte

#endif  // SCE_FORGE_CODEC_SUBBYTE_H
