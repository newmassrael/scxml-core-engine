// SCE-MAP: codec_tail:3

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_TAIL_H
#define SCE_FORGE_CODEC_TAIL_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecTail {

struct CodecTail {
    uint8_t msgId;
    uint8_t status;
    std::vector<uint8_t> payload;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecTail> decode(::SCE::Forge::SceCursor& cursor) {
        std::size_t _frame_len = cursor.remaining();
        if (_frame_len < 2) return std::nullopt;
        const std::uint8_t* raw = cursor.peek_slice(_frame_len);
        if (raw == nullptr) return std::nullopt;
        uint8_t msgId = raw[0];
        uint8_t status = raw[1];
        std::vector<uint8_t> payload = std::vector<uint8_t>(raw + 2, raw + _frame_len);
        CodecTail _decoded{
            .msgId = msgId,
            .status = status,
            .payload = payload,
        };
        if (!cursor.advance(_frame_len)) return std::nullopt;
        return _decoded;
    }

    std::vector<uint8_t> encode() const {
        std::vector<uint8_t> r;
        r.reserve(34);
        r.push_back(msgId);
        r.push_back(status);
        r.insert(r.end(), payload.begin(), payload.end());
        return r;
    }
};

}  // namespace SCE::Generated::CodecTail

#endif  // SCE_FORGE_CODEC_TAIL_H
