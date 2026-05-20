// SCE-MAP: codec_repeat_elem:5

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_REPEAT_ELEM_H
#define SCE_FORGE_CODEC_REPEAT_ELEM_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecRepeatElem {

struct CodecRepeatElem {
    uint16_t seq;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecRepeatElem> decode(::SCE::Forge::SceCursor& cursor) {
        const std::uint8_t* raw = cursor.peek_slice(2);
        if (raw == nullptr) return std::nullopt;
        uint16_t seq = static_cast<uint16_t>((static_cast<uint16_t>(raw[0]) << 8) | raw[1]);
        CodecRepeatElem value{
            .seq = seq,
        };
        if (!cursor.advance(2)) return std::nullopt;
        return value;
    }

    std::vector<uint8_t> encode() const {
        return {
            static_cast<uint8_t>((seq >> 8) & 0xFF),
            static_cast<uint8_t>(seq & 0xFF)
        };
    }
};

}  // namespace SCE::Generated::CodecRepeatElem

#endif  // SCE_FORGE_CODEC_REPEAT_ELEM_H
