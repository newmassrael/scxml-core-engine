// SCE-MAP: codec_variant_session_open:5

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_VARIANT_SESSION_OPEN_H
#define SCE_FORGE_CODEC_VARIANT_SESSION_OPEN_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecVariantSessionOpen {

struct CodecVariantSessionOpen {
    uint16_t version;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecVariantSessionOpen> decode(::SCE::Forge::SceCursor& cursor) {
        const std::uint8_t* raw = cursor.peek_slice(2);
        if (raw == nullptr) return std::nullopt;
        uint16_t version = static_cast<uint16_t>((static_cast<uint16_t>(raw[0]) << 8) | raw[1]);
        CodecVariantSessionOpen _decoded{
            .version = version,
        };
        if (!cursor.advance(2)) return std::nullopt;
        return _decoded;
    }

    std::vector<uint8_t> encode() const {
        return {
            static_cast<uint8_t>((version >> 8) & 0xFF),
            static_cast<uint8_t>(version & 0xFF)
        };
    }
};

}  // namespace SCE::Generated::CodecVariantSessionOpen

#endif  // SCE_FORGE_CODEC_VARIANT_SESSION_OPEN_H
