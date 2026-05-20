// SCE-MAP: codec_scout_zid_body:35

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_SCOUT_ZID_BODY_H
#define SCE_FORGE_CODEC_SCOUT_ZID_BODY_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecScoutZidBody {

struct CodecScoutZidBody {
    uint8_t zid_len_m1;
    std::vector<uint8_t> zid;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecScoutZidBody> decode(::SCE::Forge::SceCursor& cursor) {
        std::size_t _frame_len = cursor.remaining();
        if (_frame_len < 1) return std::nullopt;
        const std::uint8_t* raw = cursor.peek_slice(_frame_len);
        if (raw == nullptr) return std::nullopt;
        std::size_t len = _frame_len;  // shadowed for decode_expr(`raw + len`).
        (void)len;
        uint8_t zid_len_m1 = raw[0];
        std::vector<uint8_t> zid = std::vector<uint8_t>(raw + 1, raw + 1 + zid_len_m1 + 1);
        CodecScoutZidBody value{
            .zid_len_m1 = zid_len_m1,
            .zid = zid,
        };
        if (!cursor.advance(_frame_len)) return std::nullopt;
        return value;
    }

    std::vector<uint8_t> encode() const {
        std::vector<uint8_t> r;
        r.reserve(17);
        r.push_back(zid_len_m1);
        r.insert(r.end(), zid.begin(), zid.end());
        return r;
    }
};

}  // namespace SCE::Generated::CodecScoutZidBody

#endif  // SCE_FORGE_CODEC_SCOUT_ZID_BODY_H
