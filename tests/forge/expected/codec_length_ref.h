// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_LENGTH_REF_H
#define SCE_FORGE_CODEC_LENGTH_REF_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecLengthRef {

struct CodecLengthRef {
    uint8_t msgId;
    uint8_t len;
    std::vector<uint8_t> payload;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecLengthRef> decode(::SCE::Forge::SceCursor& cursor) {
        // Variable-length codec: tail / length-ref fields consume bytes
        // beyond the fixed prefix. B1-prep treats the entire cursor
        // remaining as one frame; stream-correct length-ref consumption
        // lands with its first multi-frame consumer in a later B-stage.
        std::size_t _frame_len = cursor.remaining();
        if (_frame_len < 2) return std::nullopt;
        const std::uint8_t* raw = cursor.peek_slice(_frame_len);
        if (raw == nullptr) return std::nullopt;
        std::size_t len = _frame_len;  // shadowed for decode_expr(`raw + len`).
        (void)len;
        CodecLengthRef value{
            .msgId = raw[0],
            .len = raw[1],
            .payload = std::vector<uint8_t>(raw + 2, raw + 2 + raw[1]),
        };
        if (!cursor.advance(_frame_len)) return std::nullopt;
        return value;
    }

    std::vector<uint8_t> encode() const {
        std::vector<uint8_t> r;
        r.reserve(34);
        r.push_back(msgId);
        r.push_back(len);
        r.insert(r.end(), payload.begin(), payload.end());
        return r;
    }
};

}  // namespace SCE::Generated::CodecLengthRef

#endif  // SCE_FORGE_CODEC_LENGTH_REF_H
