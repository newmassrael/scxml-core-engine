// SCE-MAP: codec_length_ref_uint32_le:13

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_LENGTH_REF_UINT32_LE_H
#define SCE_FORGE_CODEC_LENGTH_REF_UINT32_LE_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecLengthRefUint32Le {

struct CodecLengthRefUint32Le {
    uint32_t payload_len;
    std::vector<uint8_t> payload;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecLengthRefUint32Le> decode(::SCE::Forge::SceCursor& cursor) {
        std::size_t _frame_len = cursor.remaining();
        if (_frame_len < 4) return std::nullopt;
        const std::uint8_t* raw = cursor.peek_slice(_frame_len);
        if (raw == nullptr) return std::nullopt;
        std::size_t len = _frame_len;  // shadowed for decode_expr(`raw + len`).
        (void)len;
        uint32_t payload_len = static_cast<uint32_t>(raw[0] | (static_cast<uint32_t>(raw[1]) << 8) | (static_cast<uint32_t>(raw[2]) << 16) | (static_cast<uint32_t>(raw[3]) << 24));
        std::vector<uint8_t> payload = std::vector<uint8_t>(raw + 4, raw + 4 + payload_len);
        CodecLengthRefUint32Le value{
            .payload_len = payload_len,
            .payload = payload,
        };
        if (!cursor.advance(_frame_len)) return std::nullopt;
        return value;
    }

    std::vector<uint8_t> encode() const {
        std::vector<uint8_t> r;
        r.reserve(1028);
        r.push_back(static_cast<uint8_t>(payload_len & 0xFF));
        r.push_back(static_cast<uint8_t>((payload_len >> 8) & 0xFF));
        r.push_back(static_cast<uint8_t>((payload_len >> 16) & 0xFF));
        r.push_back(static_cast<uint8_t>((payload_len >> 24) & 0xFF));
        r.insert(r.end(), payload.begin(), payload.end());
        return r;
    }
};

}  // namespace SCE::Generated::CodecLengthRefUint32Le

#endif  // SCE_FORGE_CODEC_LENGTH_REF_UINT32_LE_H
