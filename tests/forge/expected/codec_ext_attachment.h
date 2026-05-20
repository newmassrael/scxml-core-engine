// SCE-MAP: codec_ext_attachment:27

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_EXT_ATTACHMENT_H
#define SCE_FORGE_CODEC_EXT_ATTACHMENT_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecExtAttachment {

struct CodecExtAttachment {
    uint8_t length;
    std::vector<uint8_t> body;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecExtAttachment> decode(::SCE::Forge::SceCursor& cursor) {
        std::size_t _frame_len = cursor.remaining();
        if (_frame_len < 1) return std::nullopt;
        const std::uint8_t* raw = cursor.peek_slice(_frame_len);
        if (raw == nullptr) return std::nullopt;
        uint8_t length = raw[0];
        std::vector<uint8_t> body = std::vector<uint8_t>(raw + 1, raw + 1 + length);
        CodecExtAttachment _decoded{
            .length = length,
            .body = body,
        };
        if (!cursor.advance(_frame_len)) return std::nullopt;
        return _decoded;
    }

    std::vector<uint8_t> encode() const {
        std::vector<uint8_t> r;
        r.reserve(65);
        r.push_back(length);
        r.insert(r.end(), body.begin(), body.end());
        return r;
    }
};

}  // namespace SCE::Generated::CodecExtAttachment

#endif  // SCE_FORGE_CODEC_EXT_ATTACHMENT_H
