// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_EXT_TIMESTAMP_H
#define SCE_FORGE_CODEC_EXT_TIMESTAMP_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecExtTimestamp {

struct CodecExtTimestamp {
    uint64_t time;
    uint8_t zid_size;
    std::vector<uint8_t> zid;

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §5.B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecExtTimestamp> decode(::SCE::Forge::SceCursor& cursor) {
        // Streaming codec: each field reads from the cursor directly
        // (VLE base-128 chain, 1..=ceil(N/7) bytes per field). RFC §5.B
        // B4: per-field bit-size dispatch routes Fixed / LengthRef
        // siblings of VLE fields through `present_if_decode_stmt`
        // (predicate=None arms) — pure-VLE codecs stay byte-stable
        // because the non-gated VLE arm there reuses
        // `vle_decode_stmt` verbatim.
        auto time_opt = cursor.read_vle_u64();
        if (!time_opt.has_value()) return std::nullopt;
        auto time = static_cast<std::uint64_t>(*time_opt);
        uint8_t zid_size;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            zid_size = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        std::vector<uint8_t> zid;
        {
            std::size_t _n = static_cast<std::size_t>(zid_size);
            const std::uint8_t* raw = cursor.peek_slice(_n);
            if (raw == nullptr) return std::nullopt;
            zid.assign(raw, raw + _n);
            if (!cursor.advance(_n)) return std::nullopt;
        }
        return CodecExtTimestamp{
            .time = time,
            .zid_size = zid_size,
            .zid = zid,
        };
    }

    std::vector<uint8_t> encode() const {
        // RFC §5.B B4: per-field bit-size dispatch routes Fixed /
        // LengthRef siblings of VLE fields through
        // `present_if_encode_block` (predicate=None arms). Pure-VLE
        // codecs stay byte-stable because the non-gated VLE arm there
        // reuses `vle_encode_block` with the language-appropriate
        // self/struct prefix.
        std::vector<uint8_t> r;
        r.reserve(28);
        {
            std::uint64_t _w = static_cast<std::uint64_t>(time);
            while (_w >= 0x80) {
                r.push_back(static_cast<std::uint8_t>((_w & 0x7F) | 0x80));
                _w >>= 7;
            }
            r.push_back(static_cast<std::uint8_t>(_w));
        }
        r.push_back(zid_size);
        r.insert(r.end(), zid.begin(), zid.end());
        return r;
    }
};

}  // namespace SCE::Generated::CodecExtTimestamp

#endif  // SCE_FORGE_CODEC_EXT_TIMESTAMP_H
