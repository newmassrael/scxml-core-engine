// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_VLE_ZINT_U64_H
#define SCE_FORGE_CODEC_VLE_ZINT_U64_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecVleZintU64 {

struct CodecVleZintU64 {
    uint64_t value;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecVleZintU64> decode(::SCE::Forge::SceCursor& cursor) {
        // Streaming codec: each field reads from the cursor directly
        // (VLE base-128 chain, 1..=ceil(N/7) bytes per field).
        auto value_opt = cursor.read_vle_u64();
        if (!value_opt.has_value()) return std::nullopt;
        auto value = static_cast<std::uint64_t>(*value_opt);
        return CodecVleZintU64{
            .value = value,
        };
    }

    std::vector<uint8_t> encode() const {
        std::vector<uint8_t> r;
        r.reserve(10);
        {
            std::uint64_t _w = static_cast<std::uint64_t>(value);
            while (_w >= 0x80) {
                r.push_back(static_cast<std::uint8_t>((_w & 0x7F) | 0x80));
                _w >>= 7;
            }
            r.push_back(static_cast<std::uint8_t>(_w));
        }
        return r;
    }
};

}  // namespace SCE::Generated::CodecVleZintU64

#endif  // SCE_FORGE_CODEC_VLE_ZINT_U64_H
