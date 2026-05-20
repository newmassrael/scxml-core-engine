// SCE-MAP: codec_variant_session_close:5

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_VARIANT_SESSION_CLOSE_H
#define SCE_FORGE_CODEC_VARIANT_SESSION_CLOSE_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecVariantSessionClose {

struct CodecVariantSessionClose {
    uint8_t reason;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecVariantSessionClose> decode(::SCE::Forge::SceCursor& cursor) {
        const std::uint8_t* raw = cursor.peek_slice(1);
        if (raw == nullptr) return std::nullopt;
        uint8_t reason = raw[0];
        CodecVariantSessionClose value{
            .reason = reason,
        };
        if (!cursor.advance(1)) return std::nullopt;
        return value;
    }

    std::vector<uint8_t> encode() const {
        return {
            reason
        };
    }
};

}  // namespace SCE::Generated::CodecVariantSessionClose

#endif  // SCE_FORGE_CODEC_VARIANT_SESSION_CLOSE_H
